use crate::modules::traits::Controller;
use crate::modules::object::Paths;

use serde_json::Value;

mod mqtt;

pub enum ControllerType {
    MQTT(mqtt::MqttController)
}

use ControllerType::*;

pub fn get_module(name: &str) -> Result<ControllerType, String> {
    return Ok(match name.to_lowercase().as_str() {
        "mqtt" => MQTT(mqtt::MqttController::new_empty()),
        unknown => {
            let msg = format!("Unknown controller module: '{}'", unknown);
            error!("{}", msg);
            return Err(msg)
        }
    })
}

impl Controller for ControllerType {
    fn init(&mut self, name: &str, config_json: &Value, paths: &Paths) -> Result<(), String> {
        return match self {
            MQTT(controller) => controller.init(name, config_json, paths)
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