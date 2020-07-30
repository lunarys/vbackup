use crate::modules::traits::Check;
use crate::util::objects::time::{TimeEntry,TimeFrame,ExecutionTiming};
use crate::util::objects::paths::{Paths, ModulePaths};
use crate::Arguments;

use serde_json::Value;
use chrono::{DateTime, Local};

mod file_age;
mod usetime;

pub enum Reference {
    Backup,
    Sync
}

pub enum CheckModule {
    FileAge(file_age::FileAge),
    Usetime(usetime::Usetime)
}

use CheckModule::*;

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

impl Check for CheckModule {
    fn init(&mut self, name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<(), String> {
        match self {
            FileAge(check) => check.init(name, config_json, paths, args),
            Usetime(check) => check.init(name, config_json, paths, args)
        }
    }

    fn check(&self, timing: &ExecutionTiming) -> Result<bool, String> {
        match self {
            FileAge(check) => check.check(timing),
            Usetime(check) => check.check(timing)
        }
    }

    fn update(&mut self, timing: &ExecutionTiming) -> Result<(), String> {
        match self {
            FileAge(check) => check.update(timing),
            Usetime(check) => check.update(timing)
        }
    }

    fn clear(&mut self) -> Result<(), String> {
        match self {
            FileAge(check) => check.clear(),
            Usetime(check) => check.clear()
        }
    }
}