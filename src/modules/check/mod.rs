use crate::modules::traits::Check;
use crate::util::objects::time::{TimeEntry,TimeFrame};
use crate::util::objects::paths::{Paths, ModulePaths};
use crate::Arguments;

use serde_json::Value;
use chrono::{DateTime, Local};

mod file_age;
mod minecraft_server;

pub enum Reference {
    Backup,
    Sync
}

pub enum CheckModule {
    FileAge(file_age::FileAge),
    MinecraftServer(minecraft_server::MinecraftServer)
}

use CheckModule::*;

pub fn get_module(name: &str) -> Result<CheckModule, String> {
    return Ok(match name.to_lowercase().as_str() {
        "file-age" => FileAge(file_age::FileAge::new_empty()),
        "minecraft-server" => MinecraftServer(minecraft_server::MinecraftServer::new_empty()),
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
            MinecraftServer(check) => check.init(name, config_json, paths, args)
        }
    }

    fn check(&self, time: &DateTime<Local>, frame: &TimeFrame, last: &Option<&TimeEntry>) -> Result<bool, String> {
        match self {
            FileAge(check) => check.check(time, frame, last),
            MinecraftServer(check) => check.check(time, frame, last)
        }
    }

    fn update(&mut self, time: &DateTime<Local>, frame: &TimeFrame, last: &Option<&TimeEntry>) -> Result<(), String> {
        match self {
            FileAge(check) => check.update(time, frame, last),
            MinecraftServer(check) => check.update(time, frame, last)
        }
    }

    fn clear(&mut self) -> Result<(), String> {
        match self {
            FileAge(check) => check.clear(),
            MinecraftServer(check) => check.clear()
        }
    }
}