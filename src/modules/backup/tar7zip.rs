use crate::modules::traits::Backup;
use crate::modules::object::*;
use crate::{try_result,try_option};

use serde_json::Value;
use serde::{Deserialize};

pub struct Tar7Zip<'a> {
    bind: Option<Bind<'a>>
}

struct Bind<'a> {
    name: &'a str,
    config: Configuration,
    paths: ModulePaths<'a>,
    dry_run: bool,
    no_docker: bool
}

#[derive(Deserialize)]
struct Configuration {
    encryption_key: Option<String>
}

impl<'a> Tar7Zip<'a> {
    pub fn new_empty() -> Self {
        return Tar7Zip { bind: None }
    }
}

impl<'a> Backup<'a> for Tar7Zip<'a> {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool) -> Result<(), String> {
        unimplemented!()
    }

    fn backup(&self, time_frames: &Vec<&TimeFrameReference>) -> Result<(), String> {
        unimplemented!()
    }

    fn restore(&self) -> Result<(), String> {
        unimplemented!()
    }

    fn clear(&mut self) -> Result<(), String> {
        try_option!(self.bind.as_ref(), "Backup is not bound, thus it can not be cleared");
        self.bind = None;
        return Ok(());
    }
}