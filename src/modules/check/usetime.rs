use crate::modules::traits::Check;
use crate::util::command::CommandWrapper;
use crate::util::io::{json,file};
use crate::{try_option,try_result,dry_run};

use serde_json::Value;
use serde::{Deserialize};
use chrono::{Local, DateTime};
use crate::util::objects::time::{TimeFrame, TimeEntry};
use crate::util::objects::paths::{Paths, ModulePaths};
use crate::Arguments;

pub struct Usetime {
    bind: Option<Bind>
}

struct Bind {
    config: Configuration,
    paths: ModulePaths,
    dry_run: bool,
    no_docker: bool
}

struct BackupInfo {
    usetime: i64,
    file_content: String
}

#[derive(Deserialize)]
struct Configuration {
    #[serde(default="relative_backup_info")]
    backup_info: String,
    targeted_usetime: i64
}

fn relative_backup_info() -> String {
    return String::from("backupinfo/props.info");
}

impl Usetime {
    pub fn new_empty() -> Self {
        return Self { bind: None };
    }
}

impl Check for Usetime {
    fn init(&mut self, _name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<(), String> {
        if self.bind.is_some() {
            let msg = String::from("Check module is already bound");
            error!("{}", msg);
            return Err(msg);
        }

        let config = json::from_value::<Configuration>(config_json.clone())?; // TODO: - clone

        self.bind = Some(Bind {
            config,
            paths,
            dry_run: args.dry_run,
            no_docker: args.no_docker
        });

        return Ok(());
    }

    fn check(&self, _time: &DateTime<Local>, _frame: &TimeFrame, last: &Option<&TimeEntry>) -> Result<bool, String> {
        let bound = try_option!(self.bind.as_ref(), "Check is not bound");

        if last.is_some() {
            let backup_info = read_backupinfo(bound)?;
            let test_result = bound.config.targeted_usetime < backup_info.usetime;

            if test_result {
                debug!("Usetime for server is larger than targeted usetime");
            } else {
                debug!("Usetime for server is smaller than targeted usetime");
            }

            return Ok(test_result);
        } else {
            debug!("There was no previous backup, additional check is not required");
            return Ok(true);
        }
    }

    fn update(&mut self, _time: &DateTime<Local>, _frame: &TimeFrame, _last: &Option<&TimeEntry>) -> Result<(), String> {
        let bound = try_option!(self.bind.as_ref(), "Check is not bound");
        let backup_info = read_backupinfo(bound)?;

        debug!("Resetting usetime for server to zero");

        if bound.dry_run {
            dry_run!(format!("Writing usetime=0 to file '{}'", backupinfo_path(bound)));
            return Ok(());
        } else {
            return reset_backupinfo(bound, &backup_info);
        }
    }

    fn clear(&mut self) -> Result<(), String> {
        try_option!(self.bind.as_ref(), "Check is not bound, thus it can not be cleared");
        self.bind = None;
        return Ok(());
    }
}

fn backupinfo_path(bind: &Bind) -> String {
    return format!("{}/{}", bind.paths.source, bind.config.backup_info);
}

fn read_backupinfo(bind: &Bind) -> Result<BackupInfo, String> {
    let file = backupinfo_path(bind);

    // TODO: Currently does not handle missing backupinfo file properly (exit with error)
    let content = if bind.no_docker {
        file::read(file.as_str())?
    } else {
        let mut cmd = CommandWrapper::new("docker");
        cmd.arg_str("run")
            .arg_str("--rm")
            .arg_string(format!("--volume={}:/file", file))
            .arg_str("--name=vbackup-tmp")
            .arg_str("alpine")
            .arg_str("sh")
            .arg_str("-c");
        cmd.arg_str("cat /file");

        if bind.dry_run {
            dry_run!(cmd.to_string());
        }

        cmd.run_get_output()?
    };

    let mut usetime: Option<i64> = None;
    for line in content.split("\n") {
        let separator_option = line.find("=");
        if separator_option.is_none() {
            continue;
        } else {
            let (key,value_tmp): (&str, &str) = line.split_at(separator_option.unwrap());
            let value = if value_tmp.starts_with("=") {
                let (_, tmp) = value_tmp.split_at(1);
                tmp
            } else {
                value_tmp
            };
            match key.to_lowercase().as_str() {
                "usetime" => {
                    debug!("Value for usetime: {}", value);
                    usetime = Some(try_result!(value.parse(), "Could not parse usetime for minecraft server"))
                }
                _ => ()
            }
        }
    }

    let result = BackupInfo {
        usetime: usetime.unwrap_or(0),
        file_content: content
    };

    return Ok(result);
}

fn reset_backupinfo(bind: &Bind, info: &BackupInfo) -> Result<(), String> {
    if bind.no_docker {
        // Use the original value to reset the usetime
        let file = backupinfo_path(bind);
        let to_replace = format!("usetime={}", info.usetime);
        let content = info.file_content.replace(to_replace.as_str(), "usetime=0");
        return file::write(file.as_str(), content.as_str(), true);
    } else {
        let mut cmd = CommandWrapper::new("docker");
        cmd.arg_str("run")
            .arg_str("--rm")
            .arg_string(format!("--volume={}:/volume", bind.paths.source))
            .arg_str("--name=vbackup-tmp")
            .arg_str("alpine")
            .arg_str("sh")
            .arg_str("-c");
        cmd.arg_string(format!("sed -i 's/usetime=.*/usetime=0/g' /volume/{}", bind.config.backup_info));
        return cmd.run();
    }
}