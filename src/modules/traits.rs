use crate::modules::object::{ModulePaths, TimeEntry, TimeFrameReference, TimeFrame};

use serde_json::Value;

pub trait Backup<'a> {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool) -> Result<(), String>;
    fn backup(&self, time_frames: &Vec<&TimeFrameReference>) -> Result<(), String>;
    fn restore(&self) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
}

pub trait Check<'a> {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool) -> Result<(), String>;
    fn check(&self, frame: &TimeFrame, last: &Option<&TimeEntry>) -> Result<bool, String>;
    fn update(&self, frame: &TimeFrame, last: &Option<&TimeEntry>) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
}

pub trait Controller<'a> {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool) -> Result<(), String>;
    fn begin(&self) -> Result<bool, String>;
    fn end(&self) -> Result<bool, String>;
    fn clear(&mut self) -> Result<(), String>;
}

pub trait Sync<'a> {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool) -> Result<(), String>;
    fn sync(&self) -> Result<(), String>;
    fn restore(&self) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
}