use crate::modules::traits::Sync;
use crate::modules::object::Paths;

use std::process::Command;
use serde_json::Value;

pub struct Duplicati {}

impl Sync for Duplicati {
    fn sync(&self, name: &String, config: &Value, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String> {
        debug!("Starting duplicati sync");

        let mut command = self.get_base_cmd(no_docker, &paths);

        let mut result = Command::new("echo")
            .arg("Hello World")
            .arg("ANother arg")
            .spawn()
            .expect("Failed");

        debug!("Duplicati sync is done");
        Ok(())
    }

    fn restore(&self, name: &String, config: &Value, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String> {
        unimplemented!()
    }
}

impl Duplicati {
    fn get_base_cmd(&self, no_docker: bool, paths: &Paths) -> Command {
        let original_path= &paths.save_path;
        let module_data = &paths.module_data_dir;

        if no_docker {
            return Command::new("duplicati-cli");
        } else {
            let mut command = Command::new("docker");
            command.arg("run")
                .arg("--rm")
                .arg("--name=vbackup-duplicati-tmp")
                .arg(format!("--volume='{}:/volume'", original_path))
                .arg(format!("--volume='{}:/dbpath'", module_data))
                .arg("duplicati/duplicati")
                .arg("duplicati-cli");
            return command;
        }
    }
}