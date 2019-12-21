use crate::modules::object::Paths;

use serde_json::Value;

pub trait Backup {
    fn backup(&self, name: &String, config: &Value, timeframes: &Value, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String>;
    fn restore(&self, name: &String, config: &Value, timeframes: &Value, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String>;
}

pub trait Check {
    fn check(&self, name: &String, config: &Value, lastsave: i64, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String>;
    fn update(&self, name: &String, config: &Value, lastsave: i64, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String>;
}

pub trait Controller {
    fn begin(&self, name: &String, config_json: &Value, paths: &Paths) -> Result<bool, String>;
    fn end(&self, name: &String, config_json: &Value, paths: &Paths) -> Result<bool, String>;
}

pub trait Sync {
    fn sync(&self, name: &String, config: &Value, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String>;
    fn restore(&self, name: &String, config: &Value, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String>;
}