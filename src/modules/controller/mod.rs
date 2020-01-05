use crate::modules::traits::Controller;
use crate::modules::object::ModulePaths;
use serde_json::Value;

mod mqtt;

pub enum ControllerModule {
    MQTT(mqtt::MqttController)
}

use ControllerModule::*;

pub fn get_module(name: &str) -> Result<ControllerModule, String> {
    return Ok(match name.to_lowercase().as_str() {
        "mqtt" => MQTT(mqtt::MqttController::new_empty()),
        unknown => {
            let msg = format!("Unknown controller module: '{}'", unknown);
            error!("{}", msg);
            return Err(msg)
        }
    })
}

impl<'a> Controller<'a> for ControllerModule {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool) -> Result<(), String> {
        return match self {
            MQTT(controller) => controller.init(name, config_json, paths, dry_run, no_docker)
        }
    }

    fn begin(&self) -> Result<bool, String> {
        return match self {
            MQTT(controller) => controller.begin()
        }
    }

    fn end(&self) -> Result<bool, String> {
        return match self {
            MQTT(controller) => controller.end()
        }
    }

    fn clear(&mut self) -> Result<(), String> {
        return match self {
            MQTT(controller) => controller.clear()
        }
    }
}