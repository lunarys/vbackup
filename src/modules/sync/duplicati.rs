use crate::modules::traits::Sync;
use crate::modules::object::Paths;

use std::process::Command;
use serde_json::Value;

pub struct Duplicati {}

impl Sync for Duplicati {
    fn sync(&self, name: &String, config: &Value, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String> {
        debug!("Starting sync");

        let mut result = Command::new("echo")
            .arg("Hello World")
            .arg("ANother arg")
            .spawn()
            .expect("Failed");

        Ok(())
    }

    fn restore(&self, name: &String, config: &Value, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String> {
        unimplemented!()
    }
}