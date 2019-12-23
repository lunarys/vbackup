use crate::modules::traits::Sync;
use crate::modules::object::{Paths, CommandWrapper};
use crate::util::auth_data;

use crate::{try_result,try_option,auth_resolve,conf_resolve};

use serde_json::Value;
use serde::{Deserialize};
use std::process::{Child, ExitStatus};
use proc_macro::quote_span;

pub struct Duplicati {}

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

impl Sync for Duplicati {
    fn sync(&self, name: &String, config_json: &Value, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String> {
        debug!("Starting duplicati sync for {}", name);

        let config : Configuration = conf_resolve!(config_json);
        let auth : Authentication = auth_resolve!(&config.auth_reference, &config.auth, paths);

        // Base command
        let mut command = get_base_cmd(no_docker, &paths);

        // Add source and destination
        command.arg_str("backup");
        command.arg_string(format!("'{}'", get_connection_uri(&config, &auth)));
        if no_docker {
            command.arg_string(format!("'{}'", &paths.save_path));
        } else {
            command.arg_str("/volume");
        }

        // Add options that are always required
        add_default_options(&mut command, name, &config, &auth, paths, no_docker);

        // Add retention options
        if config.smart_retention {
            command.arg_string(format!("--retention-policy='{}'", config.retention_policy));
        } else {
            command.arg_string(format!("--keep-versions={}", config.keep_versions));
        }

        // Additional options for backup (just for explicit declaration)
        command.arg_str("--compression-module=zip");
        command.arg_str("--encryption-module=aes");

        if dry_run {
            info!("DRY-RUN: {}", command.to_string());
        } else {
            let mut process: Child = try_result!(command.spawn(), "Starting duplicati sync failed");
            let exit_status = try_result!(process.wait(), "Duplicati sync failed");

            if !exit_status.success() {
                return Err(String::from("Duplicati backup exit code indicated error"));
            }
        }

        debug!("Duplicati sync for {} is done", name);
        Ok(())
    }

    fn restore(&self, name: &String, config_json: &Value, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String> {
        debug!("Starting duplicati restore for {}", name);

        let config : Configuration = conf_resolve!(config_json);
        let auth : Authentication = auth_resolve!(&config.auth_reference, &config.auth, paths);

        // Restore / repair the local database
        {
            let mut command = get_base_cmd(no_docker, paths);

            command.arg_str("repair");
            command.arg_string(format!("'{}'", get_connection_uri(&config, &auth)));

            add_default_options(&mut command, name, &config, &auth, paths, no_docker);

            if dry_run {
                info!("DRY-RUN: {}", command.to_string());
            } else {
                let mut process: Child = try_result!(command.spawn(), "Starting duplicati repair failed");
                let exit_status: ExitStatus = try_result!(process.wait(), "Duplicati repair failed");

                if !exit_status.success() {
                    return Err(String::from("Duplicati repair exit code indicates error"));
                }
            }
        }

        // Restore the data
        {
            let mut command = get_base_cmd(no_docker, paths);

            command.arg_str("restore");
            command.arg_string(format!("'{}'", get_connection_uri(&config, &auth)));
            command.arg_str("'*'");

            add_default_options(&mut command, name, &config, &auth, paths, no_docker);

            command.arg_str("--restore-permission=true");
            if no_docker {
                command.arg_string(format!("--restore-path='{}'", paths.save_path));
            } else {
                command.arg_str("--restore-path='/volume'");
            }

            if dry_run {
                info!("DRY-RUN: {}", command.to_string());
            } else {
                let mut process: Child = try_result!(command.spawn(), "Starting duplicati repair failed");
                let exit_status: ExitStatus = try_result!(process.wait(), "Duplicati repair failed");

                if !exit_status.success() {
                    return Err(String::from("Duplicati restore exit code indicates error"));
                }
            }
        }

        debug!("Duplicati restore for {} is done", name);
        Ok(())
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
            .arg_string(format!("--volume='{}:/dbpath'", module_data))
            .arg_str("-e AUTH_USERNAME")
            .arg_str("-e AUTH_PASSWORD")
            .arg_str("-e PASSPHRASE")
            .arg_str("duplicati/duplicati")
            .arg_str("duplicati-cli");
        return command;
    }
}

fn add_default_options(command: &mut CommandWrapper, name: &String, config: &Configuration, auth: &Authentication, paths: &Paths, no_docker: bool) {
    let dbpath = if no_docker {
        format!("{}/{}.sqlite", &paths.module_data_dir, name)
    } else {
        format!("/dbpath/{}.sqlite", name)
    };

    command.env("AUTH_USERNAME", auth.user.as_str());
    if auth.ssh_key.is_some() {
quote_span()
        if no_docker {

        } else {

        }
        // TODO: Hide! Maybe use --ssh-keyfile
        command.arg_string(format!("--ssh-key='sshkey://{}'", auth.ssh_key.as_ref().unwrap()));
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
}

fn get_connection_uri(config: &Configuration, auth: &Authentication) -> String {
    let directory_prefix = if config.directory_prefix.is_some() {
        config.directory_prefix.clone().unwrap()
    } else {
        format!("/home/{}", auth.user)
    };

    format!("ssh://{}:{}/{}/{}", auth.hostname, auth.port, directory_prefix, config.directory)
}