use std::rc::Rc;
use crate::modules::traits::Sync;
use crate::util::command::CommandWrapper;
use crate::util::io::{file,json,auth_data};
use crate::util::objects::paths::{ModulePaths,SourcePath};
use crate::Arguments;

use crate::{dry_run};

use serde_json::Value;
use serde::{Deserialize};

pub struct Duplicati {
    name: String,
    config: Configuration,
    auth: Authentication,
    paths: ModulePaths,
    dry_run: bool,
    no_docker: bool,
    print_command: bool
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

impl Sync for Duplicati {
    const MODULE_NAME: &'static str = "duplicati";

    fn new(name: &str, config_json: &Value, paths: ModulePaths, args: &Rc<Arguments>) -> Result<Box<Self>, String> {
        let config = json::from_value::<Configuration>(config_json.clone())?; // TODO: Remove clone
        let auth = auth_data::resolve::<Authentication>(&config.auth_reference, &config.auth, paths.base_paths.as_ref())?;

        warn!("Duplicati sync is deprecated! Better use borg.");

        if args.is_restore && args.restore_to.is_some() {
            return Err(String::from("The restore-to option is not supported for duplicati"));
        }

        return Ok(Box::new(Self {
            name: String::from(name),
            config,
            auth,
            paths,
            dry_run: args.dry_run,
            no_docker: args.no_docker,
            print_command: args.debug || args.verbose
        }));
    }

    fn init(&mut self) -> Result<(), String> {
        return Ok(());
    }

    fn sync(&self) -> Result<(), String> {
        debug!("Starting duplicati sync for {}", self.name);

        // Base command
        let mut command = get_base_cmd(self.no_docker, &self.paths);

        // Add source and destination
        command.arg_str("backup");
        command.arg_string(format!("{}", get_connection_uri(&self.config, &self.auth)));
        if self.no_docker {
            if let SourcePath::Single(path) = &self.paths.source {
                command.arg_string(path.clone());
            } else {
                return Err(String::from("Multiple source paths are not supported in duplicati module without docker"));
            }
        } else {
            command.arg_str("/volume");
        }

        // Add options that are always required
        add_default_options(&mut command, &self.name, &self.config, &self.auth, &self.paths, self.no_docker)?;

        // Add retention options
        if self.config.smart_retention {
            command.arg_string(format!("--retention-policy={}", self.config.retention_policy));
        } else {
            command.arg_string(format!("--keep-versions={}", self.config.keep_versions));
        }

        // Additional options for backup (just for explicit declaration)
        command.arg_str("--compression-module=zip");
        command.arg_str("--encryption-module=aes");

        if self.dry_run {
            dry_run!(command.to_string());
        } else {
            let status = if self.print_command {
                println!("-> {}", command.to_string());
                command.run_get_status()?
            } else {
                command.run_get_status_without_output()?
            };

            if let Some(code) = status.code() {
                trace!("Exit code is '{}'", code);
                if code == 50 {
                    let msg = format!("Backup uploaded some files, but did not finish");
                    error!("{}", msg);
                    return Err(msg);
                } else if code == 2 {
                    warn!("Duplicati exited with warnings");
                } else if code != 0 && code != 1 {
                    let msg = format!("Exit code indicates failure of duplicati backup");
                    error!("{}", msg);
                    return Err(msg);
                }
            }
        }

        debug!("Duplicati sync for {} is done", self.name);
        Ok(())
    }

    fn restore(&self) -> Result<(), String> {
        debug!("Starting duplicati restore for {}", self.name);

        // Restore / repair the local database
        {
            let mut command = get_base_cmd(self.no_docker, &self.paths);

            command.arg_str("repair");
            command.arg_string(format!("{}", get_connection_uri(&self.config, &self.auth)));

            add_default_options(&mut command, &self.name, &self.config, &self.auth, &self.paths, self.no_docker)?;

            command.run_configuration(self.print_command, self.dry_run)?;
        }

        // Restore the data
        {
            let mut command = get_base_cmd(self.no_docker, &self.paths);

            command.arg_str("restore");
            command.arg_string(format!("{}", get_connection_uri(&self.config, &self.auth)));
            command.arg_str("*");

            add_default_options(&mut command, &self.name, &self.config, &self.auth, &self.paths, self.no_docker)?;

            command.arg_str("--restore-permission=true");
            if self.no_docker {
                if let SourcePath::Single(path) = &self.paths.source {
                    command.arg_string(format!("--restore-path={}", path));
                } else {
                    return Err(String::from("Multiple source paths are not supported in duplicati module without docker"));
                }
            } else {
                command.arg_str("--restore-path=/volume");
            }

            command.run_configuration(self.print_command, self.dry_run)?;
        }

        debug!("Duplicati restore for {} is done", self.name);
        Ok(())
    }

    fn clear(&mut self) -> Result<(), String> {
        return Ok(());
    }
}

fn get_base_cmd(no_docker: bool, paths: &ModulePaths) -> CommandWrapper {
    if no_docker {
        return CommandWrapper::new("duplicati-cli");
    } else {
        let mut command = CommandWrapper::new("docker");
        command.arg_str("run")
            .arg_str("--rm")
            .arg_str("--name=vbackup-duplicati-tmp")
            .add_docker_volume_mapping(&paths.source, "volume")
            .arg_string(format!("--volume={}:/module", &paths.module_data_dir))
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
    let (directory_prefix, separator) = if let Some(prefix) = config.directory_prefix.as_ref() {
        let separator = if prefix.eq("") || prefix.eq("/") {
            // Putting a separating slash after an empty string or slash only makes no sense
            ""
        } else {
            "/"
        };

        (prefix.as_str(), separator)
    } else {
        ("", "")
    };

    // TODO: Check if this is handled correctly with the changed (relative prefix)
    //  There would be a second slash in the prefix if the path is not relative
    format!("ssh://{}:{}/{}{}{}", auth.hostname, auth.port, directory_prefix, separator, config.directory)
}