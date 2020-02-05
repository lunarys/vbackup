use crate::modules::traits::Sync;
use crate::modules::object::ModulePaths;
use crate::util::command::CommandWrapper;
use crate::util::io::{file,json,auth_data};

use crate::{try_result,try_option,dry_run};

use serde_json::Value;
use serde::{Deserialize};

pub struct Duplicati<'a> {
    bind: Option<Bind<'a>>
}

struct Bind<'a> {
    name: String,
    config: Configuration,
    auth: Authentication,
    paths: ModulePaths<'a>,
    dry_run: bool,
    no_docker: bool
}

#[derive(Deserialize)]
struct Configuration {
    encryption_key: Option<String>,
    directory_prefix: Option<String>,
    directory: String,
    auth_reference: Option<String>,
    auth: Option<Value>,

    #[serde(default="default_versions")]
    keep_versions: i32,

    #[serde(default="default_smart_retention")]
    smart_retention: bool,

    #[serde(default="default_retention_policy")]
    retention_policy: String,

    block_size: Option<String>, // default by duplicati: 100kb
    file_size: Option<String> // default by duplicati: 50mb
}

fn default_versions() -> i32 { 1 }
fn default_smart_retention() -> bool { false }
fn default_retention_policy() -> String { "1W:1D,4W:1W,12M:1M".to_string() }

#[derive(Deserialize)]
struct Authentication {
    hostname: String,
    port: i32,
    user: String,
    password: Option<String>,
    ssh_key: Option<String>,
    fingerprint: String
}

impl<'a> Duplicati<'a> {
    pub fn new_empty() -> Self {
        return Duplicati { bind: None };
    }
}

impl<'a> Sync<'a> for Duplicati<'a> {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool) -> Result<(), String> {
        if self.bind.is_some() {
            let msg = String::from("Sync module is already bound");
            error!("{}", msg);
            return Err(msg);
        }

        let config = json::from_value::<Configuration>(config_json.clone())?; // TODO: Remove clone
        let auth = auth_data::resolve::<Authentication>(&config.auth_reference, &config.auth, paths.base_paths)?;

        self.bind = Some(Bind {
            name: String::from(name),
            config,
            auth,
            paths,
            dry_run,
            no_docker
        });

        Ok(())
    }

    fn sync(&self) -> Result<(), String> {
        let bound = try_option!(self.bind.as_ref(), "Could not start duplicati sync, as it is not bound");

        debug!("Starting duplicati sync for {}", bound.name);

        // Base command
        let mut command = get_base_cmd(bound.no_docker, &bound.paths);

        // Add source and destination
        command.arg_str("backup");
        command.arg_string(format!("{}", get_connection_uri(&bound.config, &bound.auth)));
        if bound.no_docker {
            command.arg_string(format!("{}", &bound.paths.store_path));
        } else {
            command.arg_str("/volume");
        }

        // Add options that are always required
        add_default_options(&mut command, &bound.name, &bound.config, &bound.auth, &bound.paths, bound.no_docker)?;

        // Add retention options
        if bound.config.smart_retention {
            command.arg_string(format!("--retention-policy={}", bound.config.retention_policy));
        } else {
            command.arg_string(format!("--keep-versions={}", bound.config.keep_versions));
        }

        // Additional options for backup (just for explicit declaration)
        command.arg_str("--compression-module=zip");
        command.arg_str("--encryption-module=aes");

        if bound.dry_run {
            dry_run!(command.to_string());
        } else {
            let status = command.run_get_status()?;
            if let Some(code) = status.code() {
                if code != 0 && code != 1 {
                    let msg = format!("Exit code indicates failure of duplicati backup");
                    error!("{}", msg);
                    return Err(msg);
                }
            }
        }

        debug!("Duplicati sync for {} is done", bound.name);
        Ok(())
    }

    fn restore(&self) -> Result<(), String> {
        let bound = try_option!(self.bind.as_ref(), "Could not start duplicati restore, as it is not bound");

        debug!("Starting duplicati restore for {}", bound.name);

        // Restore / repair the local database
        {
            let mut command = get_base_cmd(bound.no_docker, &bound.paths);

            command.arg_str("repair");
            command.arg_string(format!("{}", get_connection_uri(&bound.config, &bound.auth)));

            add_default_options(&mut command, &bound.name, &bound.config, &bound.auth, &bound.paths, bound.no_docker)?;

            command.run_or_dry_run(bound.dry_run, "duplicati repair")?;
        }

        // Restore the data
        {
            let mut command = get_base_cmd(bound.no_docker, &bound.paths);

            command.arg_str("restore");
            command.arg_string(format!("{}", get_connection_uri(&bound.config, &bound.auth)));
            command.arg_str("*");

            add_default_options(&mut command, &bound.name, &bound.config, &bound.auth, &bound.paths, bound.no_docker)?;

            command.arg_str("--restore-permission=true");
            if bound.no_docker {
                command.arg_string(format!("--restore-path={}", &bound.paths.store_path));
            } else {
                command.arg_str("--restore-path=/volume");
            }

            command.run_or_dry_run(bound.dry_run, "duplicati restore")?;
        }

        debug!("Duplicati restore for {} is done", bound.name);
        Ok(())
    }

    fn clear(&mut self) -> Result<(), String> {
        let _bound = try_option!(self.bind.as_ref(), " Duplicati sync is not bound and thus can not be cleared");

        self.bind = None;
        return Ok(());
    }
}

fn get_base_cmd(no_docker: bool, paths: &ModulePaths) -> CommandWrapper {
    let original_path= &paths.store_path;
    let module_data = &paths.module_data_dir;

    if no_docker {
        return CommandWrapper::new("duplicati-cli");
    } else {
        let mut command = CommandWrapper::new("docker");
        command.arg_str("run")
            .arg_str("--rm")
            .arg_str("--name=vbackup-duplicati-tmp")
            .arg_string(format!("--volume={}:/volume", original_path))
            .arg_string(format!("--volume={}:/module", module_data))
            .arg_str("--env=AUTH_USERNAME")
            .arg_str("--env=AUTH_PASSWORD")
            .arg_str("--env=PASSPHRASE")
            .arg_str("duplicati/duplicati")
            .arg_str("duplicati-cli");
        return command;
    }
}

fn add_default_options(command: &mut CommandWrapper, name: &str, config: &Configuration, auth: &Authentication, paths: &ModulePaths, no_docker: bool) -> Result<(),String>{
    // Set username (defined as always required)
    command.env("AUTH_USERNAME", auth.user.as_str());

    // Set the name of the backup
    command.arg_string(format!("--backup-name='{}'", name));

    // Set block size option if a value is given
    // TODO: Maybe define custom default?
    if config.block_size.is_some() {
        command.arg_string(format!("--blocksize={}", config.block_size.as_ref().unwrap()));
    }

    // Set file size option if a value is given
    // TODO: Maybe define custom default?
    if config.file_size.is_some() {
        command.arg_string(format!("--dblock-size={}", config.file_size.as_ref().unwrap()));
    }

    // SSH fingerprint of host is required to securely connect
    command.arg_string(format!("--ssh-fingerprint={}", &auth.fingerprint));

    // Do not read input from console in order to prevent blocking by waiting
    command.arg_str("--disable-module=console-password-input");

    // TODO: Maybe adjust log output?
    // command.arg_with_var("--log-level=???");

    // Set encryption key is given, otherwise use no encryption
    if config.encryption_key.is_some() {
        command.env("PASSPHRASE", config.encryption_key.as_ref().unwrap())
    } else {
        command.arg_str("--no-encryption=true");
    }

    // Set dbpath (distinguish docker and no-docker run)
    command.arg_string(if no_docker {
        format!("--dbpath={}/db/{}.sqlite", &paths.module_data_dir, name)
    } else {
        format!("--dbpath=/module/db/{}.sqlite", name)
    });

    // SSH key or password options
    let identity_file_actual = format!("{}/identity", &paths.module_data_dir);
    if auth.ssh_key.is_some() {
        file::write_if_change(&identity_file_actual,
                              Some("600"),
                              auth.ssh_key.as_ref().unwrap(),
                              true)?;

        command.arg_string(if no_docker {
            format!("--ssh-keyfile={}", identity_file_actual)
        } else {
            String::from("--ssh-keyfile=/module/identity")
        });
    } else {
        if auth.password.is_some() {
            command.env("AUTH_PASSWORD", auth.password.as_ref().unwrap());
        }

        // If there was a SSH key defined at some point, remove it
        file::checked_remove(identity_file_actual.as_str())?;
    }

    Ok(())
}

fn get_connection_uri(config: &Configuration, auth: &Authentication) -> String {
    let directory_prefix = if config.directory_prefix.is_some() {
        config.directory_prefix.clone().unwrap()
    } else {
        format!("/home/{}", auth.user)
    };

    format!("ssh://{}:{}/{}/{}", auth.hostname, auth.port, directory_prefix, config.directory)
}