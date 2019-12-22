use crate::modules::traits::Sync;
use crate::modules::object::Paths;
use crate::util::auth_data;

use crate::{try_result,try_option,auth_resolve,conf_resolve};

use std::process::{Command, Child};
use serde_json::Value;
use serde::{Deserialize};

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

    #[serde(default="default_smart_retent")]
    smart_retention: bool,

    #[serde(default="default_retention_policy")]
    retention_policy: String
}

fn default_versions() -> i32 { 1 }
fn default_smart_retent() -> bool { false }
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
        debug!("Starting duplicati sync");

        let config : Configuration = conf_resolve!(config_json);
        let auth : Authentication = auth_resolve!(&config.auth_reference, &config.auth, paths);

        info!("Auth user: {}", &auth.user);

        let mut command = get_base_cmd(no_docker, &paths);
        add_default_options(&mut command, name, &config, &auth, paths, no_docker);

        /*if dry_run {
            command.to_string();
        } else {
            let mut process: Child = try_result!(command.spawn(), "Duplicati sync failed...");
            let exit_code = try_result!(process.wait(), "Duplicati sync failed");
        }*/

        debug!("Duplicati sync is done");
        Ok(())
    }

    fn restore(&self, name: &String, config_json: &Value, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String> {
        unimplemented!()
    }
}

fn get_base_cmd(no_docker: bool, paths: &Paths) -> Command {
    let original_path= &paths.save_path;
    let module_data = &paths.module_data_dir;

    if no_docker {
        return Command::new("duplicati-cli");
    } else {
        let mut command = Command::new("docker");
        command.arg("run")
            .arg("--rm")
            .arg("--name=vbackup-duplicati-tmp")
            .arg(format!("--volume='{}:/volume'", original_path))
            .arg(format!("--volume='{}:/dbpath'", module_data))
            .arg("duplicati/duplicati")
            .arg("duplicati-cli");
        return command;
    }
}

fn add_default_options(command: &mut Command, name: &String, config: &Configuration, auth: &Authentication, paths: &Paths, no_docker: bool) {
    let dbpath = if no_docker {
        format!("{}/{}.sqlite", &paths.module_data_dir, name)
    } else {
        format!("/dbpath/{}.sqlite", name)
    };

    command.arg(format!("--auth-username={}", auth.user));
    if auth.ssh_key.is_some() {
        // TODO: Hide!
        command.arg(format!("--ssh-key='{}'", auth.ssh_key.as_ref().unwrap()));
    } else if auth.password.is_some() {
        // TODO: Hide!
        command.arg(format!("--auth-password='{}'", auth.password.as_ref().unwrap()));
    }

    command.arg(format!("--dbpath='{}'", dbpath));
    command.arg(format!("--backup-name='{}'", name));

    if config.encryption_key.is_some() {
        // TODO: Hide!
        command.arg(format!("--passphrase='{}'", config.encryption_key.as_ref().unwrap()));
    } else {
        command.arg("--no-encryption=true");
    }

    command.arg(format!("--ssh-fingerprint='{}'", auth.fingerprint_rsa));
    command.arg("--disable-module=console-password-input");
    // command.arg("--log-level=???");
}

fn get_connection_uri(config: &Configuration, auth: &Authentication) -> String {
    let directory_prefix = if config.directory_prefix.is_some() {
        config.directory_prefix.clone().unwrap()
    } else {
        format!("/home/{}", auth.user)
    };

    format!("ssh://{}:{}/{}/{}", auth.hostname, auth.port, directory_prefix, config.directory)
}