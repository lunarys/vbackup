use crate::modules::traits::Sync;
use crate::modules::object::ModulePaths;
use crate::util::command::CommandWrapper;
use crate::util::auth_data;
use crate::util::io::file;

use crate::{try_result,try_option,auth_resolve,conf_resolve};

use serde_json::Value;
use serde::{Deserialize};

pub struct Rsync<'a> {
    bind: Option<Bind<'a>>
}

struct Bind<'a> {
    name: String,
    config: Configuration,
    ssh_config: SshConfig,
    paths: ModulePaths<'a>,
    sync_from: String,
    sync_to: String,
    dry_run: bool,
    no_docker: bool
}

#[derive(Deserialize)]
struct Configuration {
    #[serde(default="default_to_remote")]
    to_remote: bool,
    #[serde(default="default_compress")]
    compress: bool,
    path_prefix: Option<String>,
    dirname: String,

    host: Option<Value>,
    host_reference: Option<String>
}

fn default_to_remote() -> bool { true }
fn default_compress() -> bool { false }

#[derive(Deserialize)]
struct SshConfig {
    hostname: String,
    port: i32,
    user: String,
    password: Option<String>,
    login_key: Option<String>, // SSH private key (unencrypted)
    host_key: String // SSH public key of host
}

impl<'a> Rsync<'a> {
    pub fn new_empty() -> Self {
        return Rsync { bind: None }
    }
}

impl<'a> Sync<'a> for Rsync<'a> {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool) -> Result<(), String> {
        let config: Configuration = conf_resolve!(config_json);
        let ssh_config: SshConfig = auth_resolve!(&config.host_reference, &config.host, paths.base_paths);

        let default_path_prefix = format!("/home/{}", ssh_config.user);
        let path_prefix = config.path_prefix.as_ref().unwrap_or(&default_path_prefix);
        let remote_path = format!("{}@{}:{}",
                                  ssh_config.user,
                                  ssh_config.hostname,
                                  path_prefix);

        let (sync_from, sync_to) = if no_docker {
            if config.to_remote {
                (String::from(&paths.store_path), remote_path)
            } else {
                (format!("{}/{}", remote_path, config.dirname), String::from(&paths.store_path))
            }
        } else {
            if config.to_remote {
                (format!("/{}", config.dirname), remote_path)
            } else {
                (format!("{}/{}", remote_path, config.dirname), String::from("/"))
            }
        };

        self.bind = Some(Bind {
            name: String::from(name),
            config,
            ssh_config,
            paths,
            sync_from,
            sync_to,
            dry_run,
            no_docker
        });

        return Ok(());
    }

    fn sync(&self) -> Result<(), String> {
        let bound = try_option!(self.bind.as_ref(), "Rsync sync is not bound, it can not be used for syncing");
        let mut command = self.get_base_cmd()?;

        command.arg_string(format!("'{}'", &bound.sync_from))
            .arg_string(format!("'{}'", &bound.sync_to));

        command.run_or_dry_run(bound.dry_run, "rsync backup")?;

        return Ok(());
    }

    fn restore(&self) -> Result<(), String> {
        let bound = try_option!(self.bind.as_ref(), "Rsync is not bound, it can not be used for restoring");
        let mut command = self.get_base_cmd()?;

        command.arg_string(format!("'{}'", &bound.sync_to))
            .arg_string(format!("'{}'", &bound.sync_from));

        command.run_or_dry_run(bound.dry_run, "rsync restore")?;

        return Ok(());
    }

    fn clear(&mut self) -> Result<(), String> {
        let bound = try_option!(self.bind.as_ref(), "Rsync is not bound, thus it can not be cleared");

        self.bind = None;
        return Ok(());
    }
}

impl<'a> Rsync<'a> {
    fn get_base_cmd(&self) -> Result<CommandWrapper,String> {
        let bound = try_option!(self.bind.as_ref(), "Rsync is not bound");

        let known_hosts_file_actual = format!("{}/known_host", &bound.paths.module_data_dir);
        let identity_file_actual = format!("{}/identity", &bound.paths.module_data_dir);

        // Known host is required anyway, write it now
        file::write_if_change(&known_hosts_file_actual,
                              Some("600"),
                              &bound.ssh_config.host_key,
                              true)?;

        // Store path of known_host and identity relative to docker
        let (known_host_file, identity_file) = if bound.no_docker {
            (known_hosts_file_actual.as_str(), identity_file_actual.as_str())
        } else {
            ("/module/known_host", "/module/identity")
        };

        // Distinguish run in docker and directly on the machine
        let mut command = if bound.no_docker {
            CommandWrapper::new("rsync")
        } else {
            let mut command = CommandWrapper::new("docker");
            command.arg_str("run")
                .arg_str("--rm")
                .arg_str("--name=rsync-vbackup-tmp")
                .arg_str("--env=SSHPASS")
                .arg_string(format!("--volume='{}:{}'", &bound.paths.store_path, &bound.name));

            // Volume for authentication files
            command.arg_string(format!("--volume='{}:{}'", &bound.paths.module_data_dir, "/module"));

            // End docker command
            command.arg_str("my-rsync"); // Docker image name

            // Start rsync command
            command.arg_str("rsync");
            command
        };

        // Authentication: password or private key
        let ssh_option_end = format!("-oUserKnownHostsFile={} {}'", known_host_file, bound.ssh_config.port);
        if bound.ssh_config.login_key.is_some() {
            // SSH private key needs to be written to a file
            file::write_if_change(&identity_file_actual,
                                  Some("600"),
                                  bound.ssh_config.login_key.as_ref().unwrap(),
                                  true)?;

            // Now it can be used in the command
            command.arg_string(format!("-e 'ssh -oIdentityFile={} {}", identity_file, ssh_option_end));
        } else if bound.ssh_config.password.is_some() {
            // Use sshpass to read password as environment variable
            command.arg_string(format!("-e 'sshpass -e {}", ssh_option_end));
            command.env("SSHPASS", bound.ssh_config.password.as_ref().unwrap());
        }

        // Default sync options
        command.arg_str("--archive")
            .arg_str("--delete")
            .arg_str("--partial");

        if bound.config.compress {
            command.arg_str("--compress");
        }

        if bound.dry_run {
            command.arg_str("--dry-run");
        }

        return Ok(command);
    }
}