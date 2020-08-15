use crate::util::objects::paths::{Paths, ModulePaths};
use crate::util::objects::time::ExecutionTiming;
use crate::util::objects::reporting::ReportEvent;
use crate::Arguments;

use serde_json::Value;
use std::rc::Rc;

pub trait Backup {
    const MODULE_NAME: &'static str;
    fn get_module_name(&self) -> &str { Self::MODULE_NAME }

    fn new(name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Box<Self>, String>;
    fn init(&mut self) -> Result<(), String>;
    fn backup(&self, time_frames: &Vec<ExecutionTiming>) -> Result<(), String>;
    fn restore(&self) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
}

pub trait Check {
    const MODULE_NAME: &'static str;
    fn get_module_name(&self) -> &str { Self::MODULE_NAME }

    fn new(name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Box<Self>, String>;
    fn init(&mut self) -> Result<(), String>;
    fn check(&self, timing: &ExecutionTiming) -> Result<bool, String>;
    fn update(&mut self, timing: &ExecutionTiming) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
}

pub trait Controller {
    const MODULE_NAME: &'static str;
    fn get_module_name(&self) -> &str { Self::MODULE_NAME }

    fn new(name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Box<Self>, String>;
    fn init(&mut self) -> Result<(), String>;
    fn begin(&mut self) -> Result<bool, String>;
    fn end(&mut self) -> Result<bool, String>;
    fn clear(&mut self) -> Result<(), String>;
}

pub trait Bundleable {
    fn new_bundle(name: &str, config: &Value, paths: &Rc<Paths>, args: &Arguments) -> Result<Box<Self>,String>;
    fn try_bundle(&mut self, other_name: &str, other: &Value) -> Result<bool,String>;
}

pub trait Sync {
    const MODULE_NAME: &'static str;
    fn get_module_name(&self) -> &str { Self::MODULE_NAME }

    fn new(name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Box<Self>, String>;
    fn init(&mut self) -> Result<(), String>;
    fn sync(&self) -> Result<(), String>;
    fn restore(&self) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
}

pub trait Reporting {
    const MODULE_NAME: &'static str;
    fn get_module_name(&self) -> &str { Self::MODULE_NAME }

    fn new(config_json: &Value, paths: &Rc<Paths>, args: &Arguments) -> Result<Box<Self>, String>;
    fn init(&mut self) -> Result<(),String>;
    fn report(&self, event: ReportEvent) -> Result<(),String>;
    fn clear(&mut self) -> Result<(), String>;
}