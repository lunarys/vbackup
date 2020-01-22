use crate::modules::traits::Backup;
use crate::modules::object::{ModulePaths, TimeFrameReference};
use serde_json::Value;

mod tar7zip;

pub enum BackupModule<'a> {
    Tar7Zip(tar7zip::Tar7Zip<'a>)
}

use BackupModule::*;
use std::time::SystemTime;
use chrono::{DateTime, Local};

pub fn get_module(name: &str) -> Result<BackupModule, String> {
    return Ok(match name.to_lowercase().as_str() {
        "tar7zip" => Tar7Zip(tar7zip::Tar7Zip::new_empty()),
        unknown => {
            let msg = format!("Unknown backup module: '{}'", unknown);
            error!("{}", msg);
            return Err(msg)
        }
    })
}

impl<'a> Backup<'a> for BackupModule<'a> {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool) -> Result<(), String> {
        match self {
            Tar7Zip(backup) => backup.init(name, config_json, paths, dry_run, no_docker)
        }
    }

    fn backup(&self, time: &DateTime<Local>, time_frames: &Vec<&TimeFrameReference>) -> Result<(), String> {
        match self {
            Tar7Zip(backup) => backup.backup(time, time_frames)
        }
    }

    fn restore(&self) -> Result<(), String> {
        match self {
            Tar7Zip(backup) => backup.restore()
        }
    }

    fn clear(&mut self) -> Result<(), String> {
        match self {
            Tar7Zip(backup) => backup.clear()
        }
    }
}