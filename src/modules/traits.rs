use crate::modules::controller::ControllerModule;
use crate::util::objects::paths::{Paths, ModulePaths};
use crate::util::objects::time::ExecutionTiming;
use crate::Arguments;

use serde_json::Value;
use std::rc::Rc;

pub trait Backup {
    fn init(&mut self, name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<(), String>;
    fn backup(&self, time_frames: &Vec<ExecutionTiming>) -> Result<(), String>;
    fn restore(&self) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
}

pub trait Check {
    fn init(&mut self, name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<(), String>;
    fn check(&self, timing: &ExecutionTiming) -> Result<bool, String>;
    fn update(&mut self, timing: &ExecutionTiming) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
}

pub trait Controller {
    // This controller init is only called when the controller is used on its own (not a bundle)
    fn init(&mut self, name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<(), String>;
    fn begin(&mut self) -> Result<bool, String>;
    fn end(&mut self) -> Result<bool, String>;
    fn clear(&mut self) -> Result<(), String>;
}

pub trait Bundleable {
    // This controller init is only called when the controller is used as a bundle (before bundling)
    fn pre_init(&mut self, name: &str, config: &Value, paths: &Rc<Paths>, args: &Arguments) -> Result<(),String>;
    // In this step the bundleable controller receives all configurations for the bundle
    fn init_bundle(&mut self, modules: Vec<ControllerModule>) -> Result<(),String>;
    fn init_single(&mut self) -> Result<(),String>;
    fn can_bundle_with(&self, other: &ControllerModule) -> bool;
}

pub trait Sync {
    fn init(&mut self, name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<(), String>;
    fn sync(&self) -> Result<(), String>;
    fn restore(&self) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
}

pub trait Reporting {
    fn init(&mut self, config_json: &Value, paths: &Rc<Paths>, args: &Arguments) -> Result<(),String>;
    fn report(&self, context: Option<&[&str]>, value: &str) -> Result<(),String>;
    fn clear(&mut self) -> Result<(), String>;
}