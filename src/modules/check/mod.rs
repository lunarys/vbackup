use crate::modules::traits::Check;
use crate::util::objects::time::{ExecutionTiming};
use crate::util::objects::paths::{ModulePaths};
use crate::Arguments;

use serde_json::Value;

mod file_age;
mod usetime;

pub enum Reference {
    Backup,
    Sync
}

pub struct CheckModule {
    module: Box<dyn CheckRelay>
}

impl CheckModule {
    pub fn new(check_type: &str, name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Self,String> {
        let module: Box<dyn CheckRelay> = match check_type.to_lowercase().as_str() {
            "file-age" => file_age::FileAge::new(name, config_json, paths, args)?,
            "usetime" => usetime::Usetime::new(name, config_json, paths, args)?,
            unknown => {
                let msg = format!("Unknown check module: '{}'", unknown);
                error!("{}", msg);
                return Err(msg)
            }
        };

        return Ok(CheckModule { module });
    }
}

impl Check for CheckModule {
    fn new(_name: &str, _config_json: &Value, _paths: ModulePaths, _args: &Arguments) -> Result<Box<Self>, String> {
        return Err(String::from("Can not create anonymous check module using the default trait method"));
    }

    fn init(&mut self) -> Result<(), String> {
        self.module.init()
    }

    fn check(&self, timing: &ExecutionTiming) -> Result<bool, String> {
        self.module.check(timing)
    }

    fn update(&mut self, timing: &ExecutionTiming) -> Result<(), String> {
        self.module.update(timing)
    }

    fn clear(&mut self) -> Result<(), String> {
        self.module.clear()
    }
}

trait CheckRelay {
    fn init(&mut self) -> Result<(), String>;
    fn check(&self, timing: &ExecutionTiming) -> Result<bool, String>;
    fn update(&mut self, timing: &ExecutionTiming) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
}

impl<T: Check> CheckRelay for T {
    fn init(&mut self) -> Result<(), String> {
        Check::init(self)
    }

    fn check(&self, timing: &ExecutionTiming) -> Result<bool, String> {
        Check::check(self, timing)
    }

    fn update(&mut self, timing: &ExecutionTiming) -> Result<(), String> {
        Check::update(self, timing)
    }

    fn clear(&mut self) -> Result<(), String> {
        Check::clear(self)
    }
}