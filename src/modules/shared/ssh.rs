use serde::{Deserialize};
use crate::util::io::file;
use crate::util::command::CommandWrapper;
use crate::util::objects::paths::ModulePaths;
use std::option::Option::Some;

#[derive(Deserialize)]
pub struct SshConfig {
    pub hostname: String,
    pub port: i32,
    pub user: String,
    pub password: Option<String>,
    pub ssh_key: Option<String>, // SSH private key (unencrypted)
    pub host_key: String // SSH public key of host
}

impl CommandWrapper {
    pub fn append_ssh_command(&mut self, ssh_config: &SshConfig, module_paths: &ModulePaths, use_docker: bool, has_previous: bool) -> Result<&mut CommandWrapper, String> {
        let ssh_option_end = format!("-oUserKnownHostsFile={} -oCheckHostIp=no -p {}", get_known_hosts_filename(module_paths, use_docker), ssh_config.port);

        if ssh_config.ssh_key.is_some() {

            // The identity file needs to be created beforehand
            self.arg_string(format!("ssh -oIdentityFile={} {}", get_identity_filename(module_paths, use_docker), ssh_option_end));

        } else if let Some(password) = ssh_config.password.as_ref() {

            // Use sshpass to read password as environment variable
            //  when using docker, this needs to be passed to the container via '--env SSHPASS'
            // TODO: this is not a nice solution, maybe use a Map for storing env variables
            if !has_previous {
                self.env("SSHPASS", password);
            }

            self.arg_string(format!("sshpass -e ssh {}", ssh_option_end));
        }

        return Ok(self);
    }
}

pub fn write_known_hosts(ssh_config: &SshConfig, module_paths: &ModulePaths, dry_run: bool) -> Result<(), String> {
    if !dry_run {
        file::write_if_change(get_actual_known_hosts_filename(module_paths).as_str(), Some("600"), ssh_config.host_key.as_str(), true)?;
    }

    Ok(())
}

pub fn write_identity_file(ssh_config: &SshConfig, module_paths: &ModulePaths, dry_run: bool) -> Result<(), String> {
    if !dry_run {
        if let Some(ssh_key) = ssh_config.ssh_key.as_ref() {
            file::write_if_change(get_actual_identity_filename(module_paths).as_str(), Some("600"), ssh_key, true)?;
        }
    }

    Ok(())
}

fn get_actual_known_hosts_filename(module_paths: &ModulePaths) -> String {
    format!("{}/known_host", module_paths.module_data_dir)
}

pub fn get_known_hosts_filename(module_paths: &ModulePaths, use_docker: bool) -> String {
    if use_docker {
        String::from("/module/known_host")
    } else {
        get_actual_known_hosts_filename(module_paths)
    }
}

fn get_actual_identity_filename(module_paths: &ModulePaths) -> String {
    format!("{}/identity", module_paths.module_data_dir)
}

pub fn get_identity_filename(module_paths: &ModulePaths, use_docker: bool) -> String {
    if use_docker {
        String::from("/module/identity")
    } else {
        get_actual_identity_filename(module_paths)
    }
}