use serde::{Deserialize};
use serde_json::Value;
use crate::util::objects::paths::ModulePaths;
use crate::util::objects::time::ExecutionTiming;
use crate::Arguments;

#[derive(Deserialize)]
struct BorgConfig {
    encryption_key: Option<String>,
}

pub struct Borg {
    name: String,
    config: BorgConfig,
    paths: ModulePaths,
    dry_run: bool,
    no_docker: bool,
    print_command: bool
}

impl Borg {
    fn new(name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Box<Self>, String> {
        todo!()
    }

    fn init(&mut self) -> Result<(), String> {
        todo!()
    }

    fn clear(&mut self) -> Result<(), String> {
        todo!()
    }

    fn run_save(&self) -> Result<(), String> {
        todo!()
    }

    fn run_restore(&self) -> Result<(), String> {
        todo!()
    }
}