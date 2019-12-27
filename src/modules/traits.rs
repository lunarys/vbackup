use crate::modules::object::Paths;

use serde_json::Value;

pub trait Backup {
    fn init(&mut self, name: &str, config_json: &Value, timeframes: &Value, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String>;
    fn backup(&self) -> Result<(), String>;
    fn restore(&self) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
}

pub trait Check {
    fn init(&mut self, name: &str, config_json: &Value, lastsave: i64, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String>;
    fn check(&self) -> Result<(), String>;
    fn update(&self) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
}

pub trait Controller {
    fn init(&mut self, name: &str, config_json: &Value, paths: &Paths) -> Result<(), String>;
    fn begin(&self) -> Result<bool, String>;
    fn end(&self) -> Result<bool, String>;
    fn clear(&mut self) -> Result<(), String>;
}

pub trait Sync {
    fn init(&mut self, name: &str, config_json: &Value, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String>;
    fn sync(&self) -> Result<(), String>;
    fn restore(&self) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
}