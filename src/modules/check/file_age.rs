use crate::modules::traits::Check;
use crate::modules::object::*;
use crate::{try_result,try_option};

use serde_json::Value;
use serde::{Deserialize};

pub struct FileAge<'a> {
    bind: Option<Bind<'a>>
}

struct Bind<'a> {
    paths: ModulePaths<'a>,
    dry_run: bool,
    no_docker: bool
}

impl<'a> FileAge<'a> {
    pub fn new_empty() -> Self {
        return FileAge { bind: None };
    }
}

impl<'a> Check<'a> for FileAge<'a> {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool) -> Result<(), String> {
        if self.bind.is_some() {
            let msg = String::from("Check module is already bound");
            error!("{}", msg);
            return Err(msg);
        }

        self.bind = Some(Bind {
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