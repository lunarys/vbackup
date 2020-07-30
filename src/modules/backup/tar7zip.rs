use crate::modules::traits::Backup;
use crate::{try_option,dry_run};
use crate::util::io::{json,savefile,file};
use crate::util::command::CommandWrapper;
use crate::util::docker;

use serde_json::Value;
use serde::{Deserialize};
use std::fs::{copy, remove_file};
use chrono::{Local, DateTime};
use crate::util::objects::time::{TimeFrameReference,ExecutionTiming};
use crate::util::objects::paths::{Paths, ModulePaths};
use crate::Arguments;

pub struct Tar7Zip {
    bind: Option<Bind>
}

struct Bind {
    name: String,
    config: Configuration,
    paths: ModulePaths,
    dry_run: bool,
    no_docker: bool,
    print_command: bool
}

#[derive(Deserialize)]
struct Configuration {
    encryption_key: Option<String>
}

impl Tar7Zip {
    pub fn new_empty() -> Self {
        return Tar7Zip { bind: None }
    }
}

impl Backup for Tar7Zip {
    fn init(&mut self, name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<(), String> {
        if self.bind.is_some() {
            let msg = String::from("Backup module is already bound");
            error!("{}", msg);
            return Err(msg);
        }

        // Build local docker image
        docker::build_image_if_missing(&paths.base_paths, "p7zip.Dockerfile", "vbackup-p7zip")?;

        let config = json::from_value(config_json.clone())?; // TODO: - clone

        self.bind = Some(Bind {
            name: String::from(name),
            config,
            paths,
            dry_run: args.dry_run,
            no_docker: args.no_docker,
            print_command: args.debug || args.verbose
        });

        return Ok(());
    }

    fn backup(&self, timings: &Vec<ExecutionTiming>) -> Result<(), String> {
        let bound: &Bind = try_option!(self.bind.as_ref(), "Backup is not bound");

        let mut cmd = if bound.no_docker {
            let mut tmp = CommandWrapper::new("sh");
            tmp.arg_str("-c");
            tmp
        } else {
            let mut tmp = CommandWrapper::new("docker");
            tmp.arg_str("run")
                .arg_str("--rm")
                .arg_string(format!("--volume={}:/volume", bound.paths.source))
                .arg_string(format!("--volume={}:/savedir", bound.paths.module_data_dir))
                .arg_str("--env=ENCRYPTION_KEY")
                .arg_str("--name=vbackup-tmp")
                .arg_str("vbackup-p7zip")
                .arg_str("sh")
                .arg_str("-c");
            tmp
        };

        // Relative path to backup (if docker is used)
        let save_path = if bound.no_docker {
            bound.paths.source.as_str()
        } else {
            "/volume"
        };

        // File name for the temporary backup file
        let tmp_file_name = "vbackup-tar7zip-backup.tar.7z";
        // Path to the temporary backup file on the disk
        let tmp_backup_file_actual = format!("{}/{}", bound.paths.module_data_dir, tmp_file_name);
        // Relative path to the temporary backup file (if docker is used)
        let tmp_backup_file = if bound.no_docker {
            tmp_backup_file_actual.clone()
        } else {
            format!("/savedir/{}", tmp_file_name)
        };

        // Store the password option for 7zip, if there is no password set it to an empty String
        let password_option = if let Some(encryption_key) = bound.config.encryption_key.as_ref() {
            cmd.env("ENCRYPTION_KEY", encryption_key);
            String::from("-p\"$ENCRYPTION_KEY\" ")
        } else {
            String::new()
        };

        //  Use full path to 7z executable to avoid additional forking without the password being replaced in the process overview
        let command_actual = format!("tar -cf - -C '{}' . | /usr/lib/p7zip/7z a -si -mhe=on {}'{}'", save_path, password_option, tmp_backup_file);
        cmd.arg_string(command_actual);

        // Create a backup as temporary file
        cmd.run_configuration(bound.print_command, bound.dry_run)?;

        // Create directory for backups
        file::create_dir_if_missing(bound.paths.destination.as_str(), true)?;

        {
            let mut from: Option<String> = None;
            for timing in timings {
                let file_name = savefile::format_filename(&timing.execution_time, &timing.time_frame_reference, bound.name.as_str(), None, Some("tar.7z"));
                let backup_file = format!("{}/{}", bound.paths.destination.as_str(), file_name);

                // TODO: (?) Change permission on persisted files (currently readable by group and other due to default)?
                if from.is_none() {
                    if !bound.dry_run {
                        file::move_file(tmp_backup_file_actual.as_str(), backup_file.as_str())?;
                    } else {
                        dry_run!(format!("Moving file '{}' to '{}'", &tmp_backup_file_actual, &backup_file));
                    }
                    from = Some(backup_file);
                } else {
                    if !bound.dry_run {
                        if copy(from.as_ref().unwrap(), backup_file).is_err() {
                            error!("Could not copy temporary backup to persistent file");
                            continue;
                        }
                    } else {
                        dry_run!(format!("Copying file '{}' to '{}'", from.as_ref().unwrap(), &backup_file));
                    }
                }

                if !bound.dry_run {
                    if !savefile::prune(bound.paths.destination.as_str(), &timing.time_frame_reference.frame, &timing.time_frame_reference.amount)? {
                        trace!("Amount of backups is below threshold, not removing anything");
                    }
                } else {
                    dry_run!("Removing oldest file from backup in timeframe")
                }
            }
        }

        // Clear temporary file if still exists for some reason
        if file::exists(tmp_backup_file.as_str()) {
            if let Err(err) = remove_file(tmp_backup_file_actual) {
                error!("Could not remove temporary backup file ({})", err);
            }
        }

        Ok(())
    }

    fn restore(&self) -> Result<(), String> {
        unimplemented!()
        //let command_actual = format!("7z x -so {}'{}' | tar xf - -C '{}', password_option, backup_file, save_path);
    }

    fn clear(&mut self) -> Result<(), String> {
        try_option!(self.bind.as_ref(), "Backup is not bound, thus it can not be cleared");
        self.bind = None;
        return Ok(());
    }
}