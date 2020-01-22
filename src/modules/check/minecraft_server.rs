use crate::modules::traits::Check;
use crate::modules::object::*;
use crate::util::io::json;
use crate::{try_result,try_option};
use crate::modules::check::Reference;

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

#[derive(Deserialize)]
struct Configuration {
    serverinfo: String
}

impl<'a> MinecraftServer<'a> {
    pub fn new_empty() -> Self {
        return Self { bind: None };
    }
}

impl<'a> Check<'a> for MinecraftServer<'a> {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool, reference: Reference) -> Result<(), String> {
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

    fn check(&self, time: &DateTime<Local>, frame: &TimeFrame, last: &Option<&TimeEntry>) -> Result<bool, String> {
        unimplemented!()
    }

    fn update(&self, time: &DateTime<Local>, frame: &TimeFrame, last: &Option<&TimeEntry>) -> Result<(), String> {
        unimplemented!()
    }

    fn clear(&mut self) -> Result<(), String> {
        try_option!(self.bind.as_ref(), "Check is not bound, thus it can not be cleared");
        self.bind = None;
        return Ok(());
    }
}