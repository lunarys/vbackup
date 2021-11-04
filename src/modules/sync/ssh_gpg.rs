use crate::modules::traits::Sync;
use crate::util::objects::paths::{ModulePaths, SourcePath};
use crate::Arguments;
use crate::util::docker;
use crate::util::io::{json, auth_data, file};
use crate::util::objects::shared::ssh::SshConfig;
use crate::util::command::CommandWrapper;
use crate::{try_option,dry_run};

use serde_json::Value;
use serde::{Deserialize};
use std::borrow::Borrow;

#[derive(Deserialize)]
struct Configuration {
    encryption_key: String,
    remote_path: String,

    host: Option<Value>,
    host_reference: Option<String>,

    remote_chmod: Option<String>,
    local_chmod: Option<String>
}

pub struct SshGpg {
    name: String,
    config: Configuration,
    ssh_config: SshConfig,
    module_paths: ModulePaths,
    image: String,
    local_path: String,
    file_extension: String,
    passphrase_file: String,
    no_docker: bool,
    dry_run: bool,
    print_command: bool
}

impl Sync for SshGpg {
    const MODULE_NAME: &'static str = "ssh-gpg";

    fn new(name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Box<Self>, String> {
        let config = json::from_value::<Configuration>(config_json.clone())?; // TODO: - clone
        let ssh_config = auth_data::resolve::<SshConfig>(&config.host_reference, &config.host, paths.base_paths.as_ref())?;

        let local_path = if let SourcePath::Single(path) = paths.source.borrow() {
            path.clone()
        } else {
            let err = "Multiple source paths are not supported in this sync module";
            error!("{}", err);
            return Err(String::from(err));
        };

        return Ok(Box::new(Self {
            name: String::from(name),
            config,
            ssh_config,
            image: String::from("vbackup-gpg"),
            local_path,
            file_extension: String::from(".gpg"),
            passphrase_file: format!("{}/passphrase.txt", paths.module_data_dir),
            module_paths: paths,
            no_docker: args.no_docker,
            dry_run: args.dry_run,
            print_command: args.debug || args.verbose
        }));
    }

    fn init(&mut self) -> Result<(), String> {
        // Build local docker image
        if !self.no_docker {
            // use the rsync image for sshpass
            docker::build_image_if_missing(&self.module_paths.base_paths, "gpg.Dockerfile", self.image.as_str())?;
        }

        file::write_if_change(&self.passphrase_file, Some("600"), &self.config.encryption_key, true)?;

        // when using docker only the location inside the docker container is relevant from now on
        if !self.no_docker {
            self.passphrase_file = String::from("/module/passphrase.txt");
        }

        return Ok(());
    }

    fn sync(&self) -> Result<(), String> {
        // delete missing local from remote and copy missing remote from local
        //  cat test.txt | gpg -c --passphrase-file /tmp/password.txt --batch | ssh user@server "cat > test.txt.gpg"

        let (deleted_files, new_files) = self.find_actions()?;

        if deleted_files.is_empty() && new_files.is_empty() {
            info!("Nothing to do");
            return Ok(());
        }

        // TODO: could theoretically be optimized into one container run...

        if !deleted_files.is_empty() {
            let deleted_files_string: String = deleted_files
                .into_iter()
                .map(|file| {
                    format!("'{}{}'", file, self.file_extension)
                })
                .collect::<Vec<String>>()
                .join(" ");

            debug!("Files <{}> on the remote server are going to be deleted", deleted_files_string);

            self.get_base_cmd()
                .arg_str("sh")
                .arg_str("-c")
                .wrap()
                .append_ssh_command(&self.ssh_config, &self.module_paths, self.dry_run, !self.no_docker)?
                .arg_string(
                    format!("{}@{}", self.ssh_config.user, self.ssh_config.hostname)
                )
                .arg_string(
                    format!("\"cd '{}' && rm {}\"", self.config.remote_path, deleted_files_string)
                )
                .wrap()
                .run_configuration(self.print_command, self.dry_run)?;

            trace!("Deleted files on the remote server");
        }

        if !new_files.is_empty() {
            debug!("Files <{}> are going to be transferred to the remote server", new_files.join(" "));

            for new_file in new_files.as_slice() {
                trace!("Starting transfer for {}", new_file);

                self.get_base_cmd().arg_str("sh")
                    .arg_str("-c")
                    .wrap()
                    .arg_string(
                        format!("cat '{}/{}' |", self.local_path, new_file)
                    )
                    .arg_string(
                        format!("gpg -c --passphrase-file '{}' --batch |", self.passphrase_file)
                    )
                    .append_ssh_command(&self.ssh_config, &self.module_paths, self.dry_run, !self.no_docker)?
                    .arg_string(
                        format!("{}@{}", self.ssh_config.user, self.ssh_config.hostname)
                    )
                    .arg_string(
                        format!("\"cat > '{}/{}{}'\"", self.config.remote_path, new_file, self.file_extension)
                    )
                    .wrap()
                    .run_configuration(self.print_command, self.dry_run)?;

                trace!("Finished transfer");
            }

            trace!("All files have been transferred");

            if let Some(chmod) = self.config.remote_chmod.as_ref() {
                debug!("Setting access permissions on transferred files");

                let remote_files_string = new_files
                    .into_iter()
                    .map(|file| {
                        format!("'{}{}'", file, self.file_extension)
                    })
                    .collect::<Vec<String>>()
                    .join(" ");

                self.get_base_cmd()
                    .arg_str("sh")
                    .arg_str("-c")
                    .wrap()
                    .append_ssh_command(&self.ssh_config, &self.module_paths, self.dry_run, !self.no_docker)?
                    .arg_string(
                        format!("{}@{}", self.ssh_config.user, self.ssh_config.hostname)
                    )
                    .arg_string(
                        format!("\"cd '{}' && chmod {} {}\"", self.config.remote_path, chmod, remote_files_string)
                    )
                    .wrap()
                    .run_configuration(self.print_command, self.dry_run)?;

                trace!("Successfully set access permissions");
            }
        }

        return Ok(());
    }

    fn restore(&self) -> Result<(), String> {

        // TODO: untested

        // copy missing local from remote and keep everything else for now
        //  ssh user@server "cat test.txt.gpg" | gpg -d --passphrase-file /tmp/password.txt --batch --output file.txt

        let (missing_files, _) = self.find_actions()?;

        if missing_files.is_empty() {
            info!("Nothing to restore for {}", self.name);
            return Ok(());
        }

        debug!("Files <{}> are going to be restored from the remote server", missing_files.join(" "));

        for file in missing_files.as_slice() {
            trace!("Starting transfer for {}", file);

            self.get_base_cmd()
                .arg_str("sh")
                .arg_str("-c")
                .wrap()
                .append_ssh_command(&self.ssh_config, &self.module_paths, self.dry_run, !self.no_docker)?
                .arg_string(
                    format!("{}@{}", self.ssh_config.user, self.ssh_config.hostname)
                )
                .arg_string(
                    format!("cat '{}/{}{}'", self.config.remote_path, file, self.file_extension)
                )
                .wrap()
                .arg_string(
                    format!("| gpg -d --passphrase-file '{}' --batch --output '{}/{}'", self.passphrase_file, self.local_path, file)
                )
                .run_configuration(self.print_command, self.dry_run)?;

            trace!("Finished transfer");
        }

        trace!("All files have been restored");

        if let Some(chmod) = self.config.local_chmod.as_ref() {
            debug!("Setting access permissions on restored files");

            let files_string = missing_files
                .into_iter()
                .map(|file| format!("'{}'", file))
                .collect::<Vec<String>>()
                .join(" ");

            self.get_base_cmd()
                .arg_str("sh")
                .arg_str("-c")
                .arg_string(
                    format!("cd '{}' && chmod {} {}", self.local_path, chmod, files_string)
                )
                .run_configuration(self.print_command, self.dry_run)?;

            trace!("Successfully set access permissions");
        }

        return Ok(());
    }

    fn clear(&mut self) -> Result<(), String> {
        return Ok(());
    }
}

impl SshGpg {
    fn find_actions(&self) -> Result<(/* missing local */ Vec<String>, /* missing remote */ Vec<String>), String> {
        let local_files = self.list_local()?;
        let remote_files = self.list_remote()?.into_iter()
            .filter_map(|item|
                item.strip_suffix(&self.file_extension).map(|str| String::from(str))
            )
            .collect::<Vec<String>>();

        let local_filtered = local_files.iter().filter_map(|item| {
            if remote_files.contains(item) {
                None
            } else {
                Some(String::from(item))
            }
        }).collect::<Vec<String>>();
        let remote_filtered = remote_files.iter().filter_map(|item| {
            if local_files.contains(item) {
                None
            } else {
                Some(String::from(item))
            }
        }).collect::<Vec<String>>();

        return Ok((remote_filtered, local_filtered));
    }

    fn list_remote(&self) -> Result<Vec<String>, String> {
        let mut cmd = self.get_base_cmd();

        cmd.arg_str("sh").arg_str("-c")
            .wrap()
            .append_ssh_command(&self.ssh_config, &self.module_paths, self.dry_run, !self.no_docker)?
            .arg_string(format!("{}@{}", self.ssh_config.user, self.ssh_config.hostname));

        self.list_helper(cmd, self.config.remote_path.as_str(), false)
            .map_err(|_| String::from("Getting a list of remote files failed"))
    }

    fn list_local(&self) -> Result<Vec<String>, String> {
        let mut cmd = self.get_base_cmd();

        cmd.arg_str("sh").arg_str("-c");

        self.list_helper(cmd, self.local_path.as_str(), true)
            .map_err(|_| String::from("Getting a list of local files failed"))
    }

    fn list_helper(&self, mut base_cmd: CommandWrapper, path: &str, local: bool) -> Result<Vec<String>, String> {
        // use some random string to indicate the start of command output, in case there is some banner
        let command_start = "===== THIS IS A SEPARATOR FOR THE ACTUAL COMMAND OUTPUT =====";

        base_cmd.arg_string(format!("{2}echo {0} && mkdir -p '{1}' && ls '{1}'{2}", command_start, path, if local { "" } else { "\"" }));
        if !local {
            // the command needs to be wrapped in a single argument when running over SSH
            base_cmd.wrap();
        }

        if !local && self.dry_run {
            debug!("Retrieving a list of remote files is not possible during a dry-run, assuming an empty remote directory");
            dry_run!(base_cmd.to_string());
            Ok(vec![])
        } else {
            if self.dry_run {
                dry_run!(base_cmd.to_string());
            }

            let command_output = base_cmd.run_get_output()?;
            let result: Option<Vec<String>> = command_output.lines().fold(None, |acc,line|
                if let Some(mut values) = acc {
                    values.push(String::from(line));
                    Some(values)
                } else {
                    if line == command_start {
                        Some(vec![])
                    } else {
                        acc
                    }
                }
            );

            Ok(try_option!(result, "The listing of remote files did not return the expected separator"))
        }
    }

    fn get_base_cmd(&self) -> CommandWrapper {
        if self.no_docker {
            CommandWrapper::new_with_args("sh", vec!["-c"])
        } else {
            CommandWrapper::new_docker(
                "ssh-encrypt-vbackup-tmp",
                &self.image,
                None,
                &self.module_paths,
                (&self.module_paths.source, &self.local_path.as_str()),
                Some(vec![
                    "--env=SSHPASS"
                ])
            )
        }
    }
}