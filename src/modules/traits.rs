use crate::modules::object::{Paths, ModulePaths};

use serde_json::Value;

pub trait Backup<'a> {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, timeframes: &Value, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool) -> Result<(), String>;
    fn backup(&self) -> Result<(), String>;
    fn restore(&self) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
}

pub trait Check<'a> {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, lastsave: i64, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool) -> Result<(), String>;
    fn check(&self) -> Result<bool, String>;
    fn update(&self) -> Result<(), String>;
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