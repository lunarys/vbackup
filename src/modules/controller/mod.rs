use crate::modules::traits::{Controller, Bundleable};
use crate::util::objects::paths::{Paths, ModulePaths};
use crate::Arguments;

use serde_json::Value;
use std::rc::Rc;

pub mod bundle;
mod mqtt;

pub enum ControllerModule {
    MQTT(mqtt::MqttController),
    Bundle(bundle::ControllerBundle)
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
    fn pre_init(&mut self, name: &str, config_json: &Value, paths: &Rc<Paths>, args: &Arguments) -> Result<(), String> {
        return match self {
            MQTT(controller) => controller.pre_init(name, config_json, paths, args),
            _ => Err(String::from("pre_init called for ControllerModule that does not implement Bundleable"))
        }
    }

    fn init_bundle(&mut self, modules: Vec<ControllerModule>) -> Result<(), String> {
        return match self {
            MQTT(controller) => controller.init_bundle(modules),
            _ => Err(String::from("init_bundle called for ControllerModule that does not implement Bundleable"))
        }
    }

    fn init_single(&mut self) -> Result<(), String> {
        return match self {
            MQTT(controller) => controller.init_single(),
            _ => Err(String::from("init_single called for ControllerModule that does not implement Bundleable"))
        }
    }

    fn can_bundle_with(&self, other: &ControllerModule) -> bool {
        return match self {
            MQTT(controller) => controller.can_bundle_with(other),
            _ => false
        }
    }
}

impl ControllerModule {
    /**
      * Returns wether bundling in general is available for this type of controller module
      */
    pub fn can_bundle(&self) -> bool {
        return match self {
            MQTT(_) => true,
            _ => false
        }
    }
}