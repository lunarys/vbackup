use crate::modules::traits::Sync;
use crate::modules::object::{Paths};
use crate::util::command::CommandWrapper;
use crate::util::auth_data;
use crate::util::file;

use crate::{try_result,try_option,auth_resolve,conf_resolve};

use serde_json::Value;
use serde::{Deserialize};
use std::process::{Child, ExitStatus};

pub struct Duplicati {
    bind: Option<Bind>
}

struct Bind {
    name: String,
    config: Configuration,
    auth: Authentication,
    paths: Paths,
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
    retention_policy: String
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
    fingerprint_rsa: String
}

impl Duplicati {
    pub fn new_empty() -> Self {
        return Duplicati { bind: None };
    }
}

impl Sync for Duplicati {
    fn init(&mut self, name: &str, config_json: &Value, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String> {
        let config : Configuration = conf_resolve!(config_json);
        let auth : Authentication = auth_resolve!(&config.auth_reference, &config.auth, paths);

        self.bind = Some(Bind {
            name: String::from(name),
            config,
            auth,
            paths: paths.copy(),
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
        command.arg_string(format!("'{}'", get_connection_uri(&bound.config, &bound.auth)));
        if bound.no_docker {
            command.arg_string(format!("'{}'", &bound.paths.save_path));
        } else {
            command.arg_str("/volume");
        }

        // Add options that are always required
        add_default_options(&mut command, &bound.name, &bound.config, &bound.auth, &bound.paths, bound.no_docker)?;

        // Add retention options
        if bound.config.smart_retention {
            command.arg_string(format!("--retention-policy='{}'", bound.config.retention_policy));
        } else {
            command.arg_string(format!("--keep-versions={}", bound.config.keep_versions));
        }

        // Additional options for backup (just for explicit declaration)
        command.arg_str("--compression-module=zip");
        command.arg_str("--encryption-module=aes");

        command.run_or_dry_run(bound.dry_run, "duplicati backup")?;

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
            command.arg_string(format!("'{}'", get_connection_uri(&bound.config, &bound.auth)));

            add_default_options(&mut command, &bound.name, &bound.config, &bound.auth, &bound.paths, bound.no_docker)?;

            command.run_or_dry_run(bound.dry_run, "duplicati repair")?;
        }

        // Restore the data
        {
            let mut command = get_base_cmd(bound.no_docker, &bound.paths);

            command.arg_str("restore");
            command.arg_string(format!("'{}'", get_connection_uri(&bound.config, &bound.auth)));
            command.arg_str("'*'");

            add_default_options(&mut command, &bound.name, &bound.config, &bound.auth, &bound.paths, bound.no_docker)?;

            command.arg_str("--restore-permission=true");
            if bound.no_docker {
                command.arg_string(format!("--restore-path='{}'", &bound.paths.save_path));
            } else {
                command.arg_str("--restore-path='/volume'");
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

fn get_base_cmd(no_docker: bool, paths: &Paths) -> CommandWrapper {
    let original_path= &paths.save_path;
    let module_data = &paths.module_data_dir;

    if no_docker {
        return CommandWrapper::new("duplicati-cli");
    } else {
        let mut command = CommandWrapper::new("docker");
        command.arg_str("run")
            .arg_str("--rm")
            .arg_str("--name='vbackup-duplicati-tmp'")
            .arg_string(format!("--volume='{}:/volume'", original_path))
            .arg_string(format!("--volume='{}:/module'", module_data))
            .arg_str("-e AUTH_USERNAME")
            .arg_str("-e AUTH_PASSWORD")
            .arg_str("-e PASSPHRASE")
            .arg_str("duplicati/duplicati")
            .arg_str("duplicati-cli");
        return command;
    }
}

fn add_default_options(command: &mut CommandWrapper, name: &str, config: &Configuration, auth: &Authentication, paths: &Paths, no_docker: bool) -> Result<(), String> {
    let dbpath = if no_docker {
        format!("{}/db/{}.sqlite", &paths.module_data_dir, name)
    } else {
        format!("/module/db/{}.sqlite", name)
    };

    command.env("AUTH_USERNAME", auth.user.as_str());
    if auth.ssh_key.is_some() {
        let identity_file_actual = format!("{}/identity", &paths.module_data_dir);
        file::write_if_change(&identity_file_actual,
                              Some("600"),
                              auth.ssh_key.as_ref().unwrap(),
                              true)?;
        let keypath = if no_docker {
            identity_file_actual
        } else {
            String::from("/module/identity")
        };
        command.arg_string(format!("--ssh-keyfile='{}'", keypath));
    } else if auth.password.is_some() {
        command.env("AUTH_PASSWORD", auth.password.as_ref().unwrap());
    }

    command.arg_string(format!("--dbpath='{}'", &dbpath));
    command.arg_string(format!("--backup-name='{}'", name));

    if config.encryption_key.is_some() {
        command.env("PASSPHRASE", config.encryption_key.as_ref().unwrap())
    } else {
        command.arg_str("--no-encryption=true");
    }

    command.arg_string(format!("--ssh-fingerprint='{}'", &auth.fingerprint_rsa));
    command.arg_str("--disable-module=console-password-input");
    // command.arg_with_var("--log-level=???");

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