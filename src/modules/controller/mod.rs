use crate::modules::traits::{Controller, Bundleable};
use crate::util::objects::paths::{ModulePaths};
use crate::Arguments;
use bundle::BundleableControllerRelay;

use serde_json::Value;

pub mod bundle;
mod mqtt;
mod ping;

pub enum ControllerModule {
    Simple(Box<dyn ControllerRelay>),
    Bundle(Box<bundle::ControllerBundle>)
}

impl ControllerModule {
    pub fn new(controller_type: &str, name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Self,String> {
        let module: Box<dyn ControllerRelay> = match controller_type.to_lowercase().as_str() {
            mqtt::MqttController::MODULE_NAME => mqtt::MqttController::new(name, config_json, paths, args)?,
            ping::Ping::MODULE_NAME => ping::Ping::new(name, config_json, paths, args)?,
            unknown => {
                let msg = format!("Unknown controller module: '{}'", unknown);
                error!("{}", msg);
                return Err(msg)
            }
        };

        return Ok(ControllerModule::Simple(module))
    }

    fn as_mut_controller(&mut self) -> &mut dyn ControllerRelay {
        match self {
            ControllerModule::Simple(relay) => relay.as_mut(),
            ControllerModule::Bundle(relay) => relay.as_mut_controller()
        }
    }

    fn as_controller(&self) -> &dyn ControllerRelay {
        match self {
            ControllerModule::Simple(relay) => relay.as_ref(),
            ControllerModule::Bundle(relay) => relay.as_ref_controller()
        }
    }

    fn as_mut_bundleable(&mut self) -> Result<&mut dyn BundleableRelay, String> {
        match self {
            ControllerModule::Simple(_) => Err(String::from("Controller module does not support bundle operations")),
            ControllerModule::Bundle(relay) => Ok(relay.as_mut_bundleable())
        }
    }
}

pub trait ControllerRelay {
    fn init(&mut self) -> Result<(), String>;
    fn begin(&mut self) -> Result<bool, String>;
    fn end(&mut self) -> Result<bool, String>;
    fn clear(&mut self) -> Result<(), String>;
    fn get_module_name(&self) -> &str;
}

impl<T: Controller> ControllerRelay for T {
    fn init(&mut self) -> Result<(), String> {
        Controller::init(self)
    }

    fn begin(&mut self) -> Result<bool, String> {
        Controller::begin(self)
    }

    fn end(&mut self) -> Result<bool, String> {
        Controller::end(self)
    }

    fn clear(&mut self) -> Result<(), String> {
        Controller::clear(self)
    }

    fn get_module_name(&self) -> &str {
        Controller::get_module_name(self)
    }
}

impl ControllerRelay for ControllerModule {
    fn init(&mut self) -> Result<(), String> {
        self.as_mut_controller().init()
    }

    fn begin(&mut self) -> Result<bool, String> {
        self.as_mut_controller().begin()
    }

    fn end(&mut self) -> Result<bool, String> {
        self.as_mut_controller().end()
    }

    fn clear(&mut self) -> Result<(), String> {
        self.as_mut_controller().clear()
    }

    fn get_module_name(&self) -> &str {
        self.as_controller().get_module_name()
    }
}

pub trait BundleableRelay {
    fn try_bundle(&mut self, other_name: &str, other: &Value) -> Result<bool,String>;
    fn did_start(&self) -> bool;
}

impl<T: Bundleable> BundleableRelay for T {
    fn try_bundle(&mut self, other_name: &str, other: &Value) -> Result<bool, String> {
        self.try_bundle(other_name, other)
    }

    /*
     * Only relevant if used as a bundle, and then this is handled by bundle::ControllerBundle
     */
    fn did_start(&self) -> bool { false }
}

impl BundleableRelay for ControllerModule {
    fn try_bundle(&mut self, other_name: &str, other: &Value) -> Result<bool, String> {
        self.as_mut_bundleable()?.try_bundle(other_name, other)
    }

    fn did_start(&self) -> bool {
        match self {
            ControllerModule::Simple(_) => {false}
            ControllerModule::Bundle(bundle) => {bundle.did_start()}
        }
    }
}