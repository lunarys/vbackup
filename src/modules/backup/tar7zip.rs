use crate::modules::traits::Backup;
use crate::util::io::{json,savefile,file};
use crate::util::command::CommandWrapper;
use crate::util::docker;
use crate::util::objects::time::{ExecutionTiming};
use crate::util::objects::paths::{ModulePaths,SourcePath};
use crate::{Arguments, try_option};

use crate::{dry_run};

use serde_json::Value;
use serde::{Deserialize};
use std::fs::{copy, remove_file};
use core::borrow::{Borrow};
use std::borrow::BorrowMut;
use std::ops::Sub;
use std::path::PathBuf;
use std::rc::Rc;
use crate::util::io::user::{ask_user_abort, ask_user_option_list};

pub struct Tar7Zip {
    name: String,
    config: Configuration,
    paths: ModulePaths,
    args: Rc<Arguments>
}

#[derive(Deserialize)]
struct Configuration {
    encryption_key: Option<String>,
    #[serde(default="default_7z_executable")]
    executable: String,
    exclude: Option<Vec<String>>
}

fn default_7z_executable() -> String { String::from("7z") }

impl Backup for Tar7Zip {
    const MODULE_NAME: &'static str = "tar7zip";

    fn new(name: &str, config_json: &Value, paths: ModulePaths, args: &Rc<Arguments>) -> Result<Box<Self>, String> {
        let config = json::from_value(config_json.clone())?; // TODO: - clone
        let module = Self {
            name: String::from(name),
            config,
            paths,
            args: args.clone()
        };

        return Ok(Box::new(module));
    }

    fn init(&mut self) -> Result<(), String> {
        // Build local docker image
        if !self.args.no_docker {
            docker::build_image_if_missing(&self.paths.base_paths, "p7zip.Dockerfile", "vbackup-p7zip")?;
        }

        return Ok(());
    }

    fn backup(&self, timings: &Vec<ExecutionTiming>) -> Result<(), String> {
        let mut cmd = self.get_base_cmd(self.paths.module_data_dir.as_str());

        // Relative path to backup (if docker is used)
        let save_path = self.get_save_path()?;

        // File name for the temporary backup file
        let tmp_file_name = "vbackup-tar7zip-backup.tar.7z";
        // Path to the temporary backup file on the disk
        let tmp_backup_file_actual = format!("{}/{}", self.paths.module_data_dir, tmp_file_name);
        // Relative path to the temporary backup file (if docker is used)
        let tmp_backup_file = if self.args.no_docker {
            tmp_backup_file_actual.clone()
        } else {
            format!("/savedir/{}", tmp_file_name)
        };

        // if the temporary file already exists (e.g. from a failed / interrupted run) delete it
        if file::exists(tmp_backup_file_actual.as_str()) {
            file::remove(tmp_backup_file_actual.as_str())?;
            debug!("Deleted leftover temporary archive");
        }

        // Store the password option for 7zip, if there is no password set it to an empty String
        let password_option = self.get_password_option(cmd.borrow_mut());

        // Build to command for tar with exclude options
        let tar_exclude = self.config.exclude.as_ref().map(|exclude_list| {
            exclude_list.iter()
                .map(|exclude_part| format!("--exclude='{}'", exclude_part))
                .collect::<Vec<String>>()
                .join(" ")
        });

        let tar_command = format!("tar -cf - -C '{}' {} .", save_path, tar_exclude.as_deref().unwrap_or(""));

        // Build the command for 7zip
        //  Use full path to 7z executable to avoid additional forking without the password being replaced in the process overview
        let zip_command = format!("{} a -si -mhe=on {}'{}'", self.config.executable.as_str(), password_option, tmp_backup_file);

        // Combine the tar and the 7zip command parts
        let command_actual = format!("{} | {}", tar_command, zip_command);
        cmd.arg_string(command_actual);

        // Create a backup as temporary file
        cmd.run_with_args(self.args.as_ref())?;

        // Create directory for backups
        file::create_dir_if_missing(self.paths.destination.as_str(), true)?;

        {
            let mut from: Option<String> = None;
            for timing in timings {
                let file_name = savefile::format_filename(&timing.execution_time, &timing.time_frame_reference, self.name.as_str(), None, Some("tar.7z"));
                let backup_file = format!("{}/{}", self.paths.destination.as_str(), file_name);

                // TODO: (?) Change permission on persisted files (currently readable by group and other due to default)?
                if from.is_none() {
                    if !self.args.dry_run {
                        file::move_file(tmp_backup_file_actual.as_str(), backup_file.as_str())?;
                    } else {
                        dry_run!(format!("Moving file '{}' to '{}'", &tmp_backup_file_actual, &backup_file));
                    }
                    from = Some(backup_file);
                } else {
                    if !self.args.dry_run {
                        if copy(from.as_ref().unwrap(), backup_file).is_err() {
                            error!("Could not copy temporary backup to persistent file");
                            continue;
                        }
                    } else {
                        dry_run!(format!("Copying file '{}' to '{}'", from.as_ref().unwrap(), &backup_file));
                    }
                }

                if !self.args.dry_run {
                    if !savefile::prune(self.paths.destination.as_str(), &timing.time_frame_reference.frame, &timing.time_frame_reference.amount)? {
                        trace!("Amount of backups is below threshold, not removing anything");
                    }
                } else {
                    dry_run!("Removing oldest file from backup in timeframe");
                }
            }
        }

        // Clear temporary file if still exists for some reason
        if file::exists(tmp_backup_file_actual.as_str()) {
            if let Err(err) = remove_file(tmp_backup_file_actual) {
                error!("Could not remove temporary backup file ({})", err);
            }
        }

        return Ok(());
    }

    fn restore(&self) -> Result<(), String> {
        // find and select backup to restore
        let mut paths = file::list_in_dir(self.paths.destination.as_str())?;

        if paths.is_empty() {
            let err = "There seems to be no backup to restore";
            error!("{}", err);
            return Err(String::from(err));
        }

        // file names start with an ISO date and thus can be sorted by name
        paths.sort();

        let selected_path = ask_user_option_list(
            Some("Found multiple backup files:"),
            Some("Which backup file should be restored?"),
            paths.as_ref(),
            &|path: &PathBuf| {path.to_str().unwrap_or("<can't parse path>")},
            paths.len().sub(1)
        )?;

        let chosen_file = try_option!(selected_path.file_name().map(|name| name.to_str()).flatten(), "Could not read filename for latest file");

        ask_user_abort(Some(&format!("Continue to restore '{}'?", chosen_file)))?;

        // Relative path to restore (if docker is used)
        let contextual_restore_path = self.get_save_path()?;
        let contextual_backup_path = if self.args.no_docker {
            format!("{}/{}", self.paths.destination.as_str(), chosen_file)
        } else {
            format!("/savedir/{}", chosen_file)
        };

        let mut cmd = self.get_base_cmd(self.paths.destination.as_str());

        // Store the password option for 7zip, if there is no password set it to an empty String
        let password_option = self.get_password_option(cmd.borrow_mut());

        //let command_actual = format!("7z x -so {}'{}' | tar xf - -C '{}', password_option, backup_file, save_path);
        let unzip_command = format!("7z x -so {}'{}'", password_option, contextual_backup_path);
        let untar_command = format!("tar xf - -C '{}'", contextual_restore_path);
        let command_actual = format!("{} | {}", unzip_command, untar_command);

        cmd.arg_string(command_actual);

        info!("Starting restore of '{}'...", chosen_file);
        cmd.run_with_args(self.args.as_ref())?;
        info!("Restore done.");

        Ok(())
    }

    fn clear(&mut self) -> Result<(), String> {
        return Ok(());
    }
}

impl Tar7Zip {
    fn get_base_cmd(&self, mount_path: &str) -> CommandWrapper {
        return if self.args.no_docker {
            let mut cmd = CommandWrapper::new("sh");
            cmd.arg_str("-c");
            cmd
        } else {
            let mut cmd = CommandWrapper::new("docker");

            let mut source_overwrite = None;
            if self.args.is_restore {
                if let Some(restore_to) = self.args.restore_to.as_ref() {
                    source_overwrite.replace(SourcePath::Single(restore_to.clone()));
                }
            }

            let source_mount = source_overwrite.as_ref().unwrap_or(self.paths.source.borrow());

            cmd.arg_str("run")
                .arg_str("--rm")
                .add_docker_volume_mapping(source_mount, "volume")
                .arg_string(format!("--volume={}:/savedir", mount_path))
                .arg_str("--env=ENCRYPTION_KEY")
                .arg_str("--name=vbackup-tmp")
                .arg_str("vbackup-p7zip")
                .arg_str("sh")
                .arg_str("-c");
            cmd
        };
    }

    fn get_save_path(&self) -> Result<&str, String> {
        return if self.args.no_docker {
            if self.args.is_restore && self.args.restore_to.is_some() {
                Ok(self.args.restore_to.as_ref().unwrap())
            } else if let SourcePath::Single(path) = &self.paths.source {
                Ok(path.as_str())
            } else {
                Err(String::from("Multiple source paths are not supported in tar7zip module without docker"))
            }
        } else {
            Ok("/volume")
        }
    }

    fn get_password_option(&self, cmd: &mut CommandWrapper) -> String {
        // Store the password option for 7zip, if there is no password set it to an empty String
        return if let Some(encryption_key) = self.config.encryption_key.as_ref() {
            cmd.env("ENCRYPTION_KEY", encryption_key);
            String::from("-p\"$ENCRYPTION_KEY\" ")
        } else {
            String::new()
        };
    }
}