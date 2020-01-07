use crate::modules::traits::Check;
use crate::modules::object::*;
use crate::{try_result,try_option,auth_resolve,conf_resolve};

use serde_json::Value;
use serde::{Deserialize};

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
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool) -> Result<(), String> {
        let config: Configuration = conf_resolve!(config_json);

        self.bind = Some(Bind {
            config,
            paths,
            dry_run,
            no_docker
        });

        return Ok(());
    }

    fn check(&self, frame: &TimeFrame, last: &Option<&TimeEntry>) -> Result<bool, String> {
        unimplemented!()
    }

    fn update(&self, frame: &TimeFrame, last: &Option<&TimeEntry>) -> Result<(), String> {
        unimplemented!()
    }

    fn clear(&mut self) -> Result<(), String> {
        try_option!(self.bind.as_ref(), "Check is not bound, thus it can not be cleared");
        self.bind = None;
        return Ok(());
    }
}