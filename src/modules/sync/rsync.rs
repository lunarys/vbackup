use crate::modules::traits::Sync;
use crate::modules::object::{ModulePaths,Arguments};
use crate::util::command::CommandWrapper;
use crate::util::io::{file,json,auth_data};
use crate::util::docker;

use crate::{try_option};

use serde_json::Value;
use serde::{Deserialize};

pub struct Rsync<'a> {
    bind: Option<Bind<'a>>
}

struct Bind<'a> {
    _name: String,
    config: Configuration,
    ssh_config: SshConfig,
    paths: ModulePaths<'a>,
    sync_from: String,
    sync_to: String,
    dry_run: bool,
    no_docker: bool,
    verbose: bool,
    print_command: bool
}

#[derive(Deserialize)]
struct Configuration {
    #[serde(default="default_true")]
    to_remote: bool,
    #[serde(default="default_false")]
    compress: bool,
    path_prefix: Option<String>,
    dirname: String,

    host: Option<Value>,
    host_reference: Option<String>
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }

#[derive(Deserialize)]
struct SshConfig {
    hostname: String,
    port: i32,
    user: String,
    password: Option<String>,
    ssh_key: Option<String>, // SSH private key (unencrypted)
    host_key: String // SSH public key of host
}

impl<'a> Rsync<'a> {
    pub fn new_empty() -> Self {
        return Rsync { bind: None }
    }
}

impl<'a> Sync<'a> for Rsync<'a> {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths<'b>, args: &Arguments) -> Result<(), String> {
        if self.bind.is_some() {
            let msg = String::from("Sync module is already bound");
            error!("{}", msg);
            return Err(msg);
        }

        // Build local docker image if missing
        docker::build_image_if_missing(&paths.base_paths, "rsync.Dockerfile", "vbackup-rsync")?;

        let config = json::from_value::<Configuration>(config_json.clone())?; // TODO: - clone
        let ssh_config = auth_data::resolve::<SshConfig>(&config.host_reference, &config.host, paths.base_paths)?;

        let default_path_prefix = format!("/home/{}", ssh_config.user);
        let path_prefix = config.path_prefix.as_ref().unwrap_or(&default_path_prefix);
        let remote_path = format!("{}@{}:{}",
                                  ssh_config.user,
                                  ssh_config.hostname,
                                  path_prefix);

        let (sync_from, sync_to) = if args.no_docker {
            if config.to_remote {
                (paths.source.clone(), remote_path)
            } else {
                (format!("{}/{}", remote_path, config.dirname), paths.source.clone())
            }
        } else {
            if config.to_remote {
                (format!("/{}", config.dirname), remote_path)
            } else {
                (format!("{}/{}", remote_path, config.dirname), String::from("/"))
            }
        };

        self.bind = Some(Bind {
            _name: String::from(name),
            config,
            ssh_config,
            paths,
            sync_from,
            sync_to,
            dry_run: args.dry_run,
            no_docker: args.no_docker,
            verbose: args.verbose,
            print_command: args.debug || args.verbose
        });

        return Ok(());
    }

    fn sync(&self) -> Result<(), String> {
        let bound = try_option!(self.bind.as_ref(), "Rsync sync is not bound, it can not be used for syncing");
        let mut command = self.get_base_cmd()?;

        command.arg_string(format!("{}", &bound.sync_from))
            .arg_string(format!("{}", &bound.sync_to));

        command.run_configuration(bound.print_command, bound.dry_run)?;

        return Ok(());
    }

    fn restore(&self) -> Result<(), String> {
        let bound = try_option!(self.bind.as_ref(), "Rsync is not bound, it can not be used for restoring");
        let mut command = self.get_base_cmd()?;

        command.arg_string(format!("{}", &bound.sync_to))
            .arg_string(format!("{}", &bound.sync_from));

        command.run_configuration(bound.print_command, bound.dry_run)?;

        return Ok(());
    }

    fn clear(&mut self) -> Result<(), String> {
        let _bound = try_option!(self.bind.as_ref(), "Rsync is not bound, thus it can not be cleared");

        self.bind = None;
        return Ok(());
    }
}

impl<'a> Rsync<'a> {
    fn get_base_cmd(&self) -> Result<CommandWrapper,String> {
        let bound = try_option!(self.bind.as_ref(), "Rsync is not bound");

        if !bound.dry_run {
            file::create_dir_if_missing(bound.paths.module_data_dir.as_str(), true)?;
        }

        let known_hosts_file_actual = format!("{}/known_host", &bound.paths.module_data_dir);
        let identity_file_actual = format!("{}/identity", &bound.paths.module_data_dir);

        // Known host is required anyway, write it now
        if !bound.dry_run {
            file::write_if_change(&known_hosts_file_actual,
                                  Some("600"),
                                  &bound.ssh_config.host_key,
                                  true)?;
        }

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
                .arg_string(format!("--volume={}:/{}", &bound.paths.source, &bound.config.dirname));

            // Volume for authentication files
            command.arg_string(format!("--volume={}:{}", &bound.paths.module_data_dir, "/module"));

            // End docker command
            command.arg_str("vbackup-rsync"); // Docker image name

            // Start rsync command
            command.arg_str("rsync");
            command
        };

        // Authentication: password or private key
        command.arg_str("-e");
        let ssh_option_end = format!("-oUserKnownHostsFile={} -oCheckHostIp=no -p {}", known_host_file, bound.ssh_config.port);
        if bound.ssh_config.ssh_key.is_some() {
            // SSH private key needs to be written to a file
            if !bound.dry_run {
                file::write_if_change(&identity_file_actual,
                                      Some("600"),
                                      bound.ssh_config.ssh_key.as_ref().unwrap(),
                                      true)?;
            }

            // Now it can be used in the command
            command.arg_string(format!("ssh -oIdentityFile={} {}", identity_file, ssh_option_end));
        } else if bound.ssh_config.password.is_some() {
            // Use sshpass to read password as environment variable
            command.arg_string(format!("sshpass -e ssh {}", ssh_option_end));
            command.env("SSHPASS", bound.ssh_config.password.as_ref().unwrap());
        }

        // Default sync options
        command.arg_str("--archive")
            .arg_str("--delete")
            .arg_str("--partial");

        // TODO: Create an option for this?
        // Set file permissions for receiving end
        if bound.config.to_remote {
            command.arg_str("--chmod=ug=rwX,o-rwx");
            command.arg_str("--perms");
        }

        if bound.config.compress {
            command.arg_str("--compress");
        }

        if bound.dry_run {
            command.arg_str("--dry-run");
        }

        if bound.verbose {
            command.arg_str("--verbose");
        }

        return Ok(command);
    }
}