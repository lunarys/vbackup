use crate::modules::traits::{Controller, Bundleable};
use crate::modules::object::{ModulePaths, Arguments, Paths};
use serde_json::Value;

pub mod bundle;
mod mqtt;

pub enum ControllerModule {
    MQTT(mqtt::MqttController),
    Bundle(bundle::ControllerBundle)
}

use ControllerModule::*;
use std::rc::Rc;

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

impl Controller for ControllerModule {
    fn init(&mut self, name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<(), String> {
        return match self {
            MQTT(controller) => Controller::init(controller, name, config_json, paths, args),
            Bundle(controller) => controller.init(name, config_json, paths, args)
        }
    }

    fn begin(&mut self) -> Result<bool, String> {
        return match self {
            MQTT(controller) => controller.begin(),
            Bundle(controller) => controller.begin()
        }
    }

    fn end(&mut self) -> Result<bool, String> {
        return match self {
            MQTT(controller) => controller.end(),
            Bundle(controller) => controller.end()
        }
    }

    fn clear(&mut self) -> Result<(), String> {
        return match self {
            MQTT(controller) => controller.clear(),
            Bundle(controller) => controller.clear()
        }
    }
}

impl Bundleable for ControllerModule {
    fn init(&mut self, name: &str, config_json: &Value, paths: &Rc<Paths>, args: &Arguments) -> Result<(), String> {
        unimplemented!()
    }

    fn update_module_paths(&mut self, paths: ModulePaths) -> Result<(), String> {
        unimplemented!()
    }

    fn can_bundle_with(&self, other: &ControllerModule) -> bool {
        unimplemented!()
    }
}

impl ControllerModule {
    pub fn can_bundle(&self) -> bool {
        return match self {
            _ => false
        }
    }
}