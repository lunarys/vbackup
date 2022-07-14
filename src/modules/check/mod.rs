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
    module: Box<dyn CheckWrapper>
}

impl CheckModule {
    pub fn new(check_type: &str, name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Self,String> {
        let module: Box<dyn CheckWrapper> = match check_type.to_lowercase().as_str() {
            file_age::FileAge::MODULE_NAME => file_age::FileAge::new(name, config_json, paths, args)?,
            usetime::Usetime::MODULE_NAME => usetime::Usetime::new(name, config_json, paths, args)?,
            unknown => {
                let msg = format!("Unknown check module: '{}'", unknown);
                error!("{}", msg);
                return Err(msg)
            }
        };

        return Ok(CheckModule { module });
    }
}

impl CheckWrapper for CheckModule {
    fn init(&mut self) -> Result<(), String> {
        self.module.init()
    }

    fn check(&mut self, timing: &ExecutionTiming) -> Result<bool, String> {
        self.module.check(timing)
    }

    fn update(&mut self, timing: &ExecutionTiming) -> Result<(), String> {
        self.module.update(timing)
    }

    fn clear(&mut self) -> Result<(), String> {
        self.module.clear()
    }

    fn get_module_name(&self) -> &str {
        self.module.get_module_name()
    }
}

pub trait CheckWrapper {
    fn init(&mut self) -> Result<(), String>;
    fn check(&mut self, timing: &ExecutionTiming) -> Result<bool, String>;
    fn update(&mut self, timing: &ExecutionTiming) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
    fn get_module_name(&self) -> &str;
}

impl<T: Check> CheckWrapper for T {
    fn init(&mut self) -> Result<(), String> {
        Check::init(self)
    }

    fn check(&mut self, timing: &ExecutionTiming) -> Result<bool, String> {
        Check::check(self, timing)
    }

    fn update(&mut self, timing: &ExecutionTiming) -> Result<(), String> {
        Check::update(self, timing)
    }

    fn clear(&mut self) -> Result<(), String> {
        Check::clear(self)
    }

    fn get_module_name(&self) -> &str {
        Check::get_module_name(self)
    }
}