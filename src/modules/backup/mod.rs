use crate::modules::traits::Backup;
use crate::util::objects::time::TimeFrameReference;
use crate::util::objects::paths::{Paths, ModulePaths};
use crate::Arguments;

use serde_json::Value;
use chrono::{DateTime, Local};

mod tar7zip;

pub enum BackupModule {
    Tar7Zip(tar7zip::Tar7Zip)
}

use BackupModule::*;

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

impl Backup for BackupModule {
    fn init(&mut self, name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<(), String> {
        match self {
            Tar7Zip(backup) => backup.init(name, config_json, paths, args)
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