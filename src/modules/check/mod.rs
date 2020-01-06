use crate::modules::traits::Check;
use crate::modules::object::{ModulePaths,TimeEntry, TimeFrame};
use serde_json::Value;

pub enum CheckModule {
    NotImplemented
}

use CheckModule::*;

pub fn get_module(name: &str) -> Result<CheckModule, String> {
    return Ok(match name.to_lowercase().as_str() {
        //"mqtt" => MQTT(mqtt::MqttController::new_empty()),
        unknown => {
            let msg = format!("Unknown controller module: '{}'", unknown);
            error!("{}", msg);
            return Err(msg)
        }
    })
}

impl<'a> Check<'a> for CheckModule {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths, dry_run: bool, no_docker: bool) -> Result<(), String> {
        unimplemented!()
    }

    fn check(&self, frame: &TimeFrame, last: &Option<&TimeEntry>) -> Result<bool, String> {
        unimplemented!()
    }

    fn update(&self, frame: &TimeFrame, last: &Option<&TimeEntry>) -> Result<(), String> {
        unimplemented!()
    }

    fn clear(&mut self) -> Result<(), String> {
        unimplemented!()
    }
}