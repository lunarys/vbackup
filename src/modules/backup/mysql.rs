use crate::modules::traits::Backup;
use crate::modules::object::*;
use crate::{try_option,dry_run};
use crate::util::io::{json,savefile,file};
use crate::util::command::CommandWrapper;
use crate::util::docker;

use serde_json::Value;
use serde::{Deserialize};
use chrono::{Local, DateTime};

pub struct Module<'a> {
    bind: Option<Bind<'a>>
}

struct Bind<'a> {
    name: String,
    config: Configuration,
    paths: ModulePaths<'a>,
    dry_run: bool,
    no_docker: bool,
    print_command: bool
}

#[derive(Deserialize)]
struct Configuration {
    user: String,
    password: String,
    host: String,
    port: u32,
    databases: Vec<String>,
    encryption_key: Option<String>
}

impl<'a> Module<'a> {
    pub fn new_empty() -> Self {
        return Module { bind: None }
    }
}

impl<'a> Backup<'a> for Module<'a> {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<(), String> {
        unimplemented!()
    }

    fn backup(&self, time: &DateTime<Local>, time_frames: &Vec<&TimeFrameReference>) -> Result<(), String> {
        unimplemented!()
    }

    fn restore(&self) -> Result<(), String> {
        unimplemented!()
    }

    fn clear(&mut self) -> Result<(), String> {
        unimplemented!()
    }
}