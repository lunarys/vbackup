use crate::modules::traits::Check;
use crate::modules::object::{ModulePaths,TimeEntry,TimeFrame,Arguments};
use serde_json::Value;

mod file_age;
mod usetime;

pub enum Reference {
    Backup,
    Sync
}

pub enum CheckModule<'a> {
    FileAge(file_age::FileAge<'a>),
    Usetime(usetime::Usetime<'a>)
}

use CheckModule::*;
use chrono::{DateTime, Local};

pub fn get_module(name: &str) -> Result<CheckModule, String> {
    return Ok(match name.to_lowercase().as_str() {
        "file-age" => FileAge(file_age::FileAge::new_empty()),
        "usetime" => Usetime(usetime::Usetime::new_empty()),
        unknown => {
            let msg = format!("Unknown check module: '{}'", unknown);
            error!("{}", msg);
            return Err(msg)
        }
    })
}

impl<'a> Check<'a> for CheckModule<'a> {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths<'b>, args: &Arguments) -> Result<(), String> {
        match self {
            FileAge(check) => check.init(name, config_json, paths, args),
            Usetime(check) => check.init(name, config_json, paths, args)
        }
    }

    fn check(&self, time: &DateTime<Local>, frame: &TimeFrame, last: &Option<&TimeEntry>) -> Result<bool, String> {
        match self {
            FileAge(check) => check.check(time, frame, last),
            Usetime(check) => check.check(time, frame, last)
        }
    }

    fn update(&self, time: &DateTime<Local>, frame: &TimeFrame, last: &Option<&TimeEntry>) -> Result<(), String> {
        match self {
            FileAge(check) => check.update(time, frame, last),
            Usetime(check) => check.update(time, frame, last)
        }
    }

    fn clear(&mut self) -> Result<(), String> {
        match self {
            FileAge(check) => check.clear(),
            Usetime(check) => check.clear()
        }
    }
}