use crate::modules::traits::Check;
use crate::modules::object::*;
use crate::util::command::CommandWrapper;
use crate::util::io::{json,file};
use crate::{try_option,try_result,dry_run};

use serde_json::Value;
use serde::{Deserialize};
use chrono::{Local, DateTime};

pub struct MinecraftServer<'a> {
    bind: Option<Bind<'a>>
}

struct Bind<'a> {
    config: Configuration,
    paths: ModulePaths<'a>,
    dry_run: bool,
    no_docker: bool
}

struct BackupInfo {
    usetime: i64,
    save: bool
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

impl<'a> MinecraftServer<'a> {
    pub fn new_empty() -> Self {
        return Self { bind: None };
    }
}

impl<'a> Check<'a> for MinecraftServer<'a> {
    fn init<'b: 'a>(&mut self, _name: &str, config_json: &Value, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool) -> Result<(), String> {
        if self.bind.is_some() {
            let msg = String::from("Check module is already bound");
            error!("{}", msg);
            return Err(msg);
        }

        let config = json::from_value::<Configuration>(config_json.clone())?; // TODO: - clone

        self.bind = Some(Bind {
            config,
            paths,
            dry_run,
            no_docker
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

    fn update(&self, _time: &DateTime<Local>, _frame: &TimeFrame, _last: &Option<&TimeEntry>) -> Result<(), String> {
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
    let content = if bind.no_docker {
        file::read(file.as_str())?
    } else {
        let mut cmd = CommandWrapper::new("docker");
        cmd.arg_str("run")
            .arg_str("--rm")
            .arg_string(format!("--volume={}:/volume", file))
            .arg_str("--name=vbackup-tmp")
            .arg_str("alpine")
            .arg_str("sh")
            .arg_str("-c");
        cmd.arg_string(format!("cat /volume/{}", bind.config.backup_info));
        cmd.run_get_output()?
    };

    let mut result = BackupInfo {
        usetime: 0,
        save: true
    };

    for line in content.split("\n") {
        let separator_option = line.find("=");
        if separator_option.is_none() {
            continue;
        } else {
            let (key,value): (&str, &str) = line.split_at(separator_option.unwrap());
            match key.to_lowercase().as_str() {
                "usetime" => {
                    result.usetime = try_result!(value.parse(), "Could not parse usetime for minecraft server")
                },
                "save" => {
                    result.save = value.to_lowercase() == "true"
                },
                _ => ()
            }
        }
    }

    return Ok(result);
}

fn reset_backupinfo(bind: &Bind, info: &BackupInfo) -> Result<(), String> {
    let file = backupinfo_path(bind);
    let content = format!("usetime={}\nsave={}", 0, info.save);

    if bind.no_docker {
        return file::write(file.as_str(), content.as_str(), true);
    } else {
        let mut cmd = CommandWrapper::new("docker");
        cmd.arg_str("run")
            .arg_str("--rm")
            .arg_string(format!("--volume={}:/volume", file))
            .arg_str("--name=vbackup-tmp")
            .arg_str("alpine")
            .arg_str("sh")
            .arg_str("-c");
        cmd.arg_string(format!("echo \"{}\" > /volume/{}", content, bind.config.backup_info));
        return cmd.run();
    }
}