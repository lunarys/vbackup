use crate::modules::controller::{ControllerModule, ControllerWrapper, BundleableWrapper};
use crate::modules::controller::mqtt::MqttController;
use crate::modules::traits::{Controller,Bundleable};
use crate::util::objects::paths::Paths;
use crate::Arguments;

use serde_json::Value;
use std::rc::Rc;

pub struct ControllerBundle {
    controller: Box<dyn BundleableControllerWrapper>,
    init_result: Option<Result<(),String>>,
    begin_result: Option<Result<bool,String>>,
    bundled: bool
}

impl ControllerBundle {
    pub fn new(controller_type: &str, name: &str, config: &Value, paths: &Rc<Paths>, args: &Arguments) -> Result<ControllerBundle,String> {
        let module: Box<dyn BundleableControllerWrapper> = match controller_type.to_lowercase().as_str() {
            MqttController::MODULE_NAME => {
                MqttController::new_bundle(name, config, paths, args)?
            },
            unknown => {
                let msg = format!("Unknown or unbundleable controller module: '{}'", unknown);
                error!("{}", msg);
                return Err(msg)
            }
        };

        return Ok(ControllerBundle {
            controller: module,
            init_result: None,
            begin_result: None,
            bundled: false
        });
    }

    pub fn into_simple_controller(self) -> Result<ControllerModule,String> {
        if self.bundled || self.begin_result.is_some() {
            return Err(String::from("Can't move controller bundle that is bundled or initiated into simple controller"));
        } else {
            return Ok(ControllerModule::Simple(self.controller.into_controller()));
        }
    }

    pub fn into_controller(self) -> ControllerModule {
        return ControllerModule::Bundle(Box::new(self));
    }

    pub fn done(&mut self) -> Result<(),String> {
        // TODO: Handle results
        let end_result = self.controller.end();
        let clear_result = self.controller.clear();
        return end_result.and(clear_result);
    }

    /**
      * Returns wether bundling in general is available for this type of controller module
      */
    pub fn _can_bundle(&self) -> bool {
        return ControllerBundle::can_bundle_type(self.get_module_name());
    }

    pub fn can_bundle_type(name: &str) -> bool {
        return match name {
            MqttController::MODULE_NAME => true,
            _ => false
        }
    }
}

impl ControllerWrapper for ControllerBundle {
    fn init(&mut self) -> Result<(), String> {
        if let Some(result) = self.init_result.as_ref() {
            return result.clone();
        } else {
            let result = self.controller.init();
            self.init_result = Some(result.clone());
            return result;
        }
    }

    fn begin(&mut self) -> Result<bool, String> {
        if let Some(result) = self.begin_result.as_ref() {
            return result.clone();
        } else {
            let result = self.controller.begin();
            self.begin_result = Some(result.clone());
            return result;

        }
    }

    fn end(&mut self) -> Result<bool, String> {
        // dummy, real end is in 'done'
        return Ok(true);
    }

    fn clear(&mut self) -> Result<(), String> {
        // dummy, real clear is in 'done'
        return Ok(());
    }

    fn get_module_name(&self) -> &str {
        self.controller.get_module_name()
    }
}

impl BundleableWrapper for ControllerBundle {
    fn try_bundle(&mut self, other_name: &str, other: &Value) -> Result<bool, String> {
        let result = self.controller.as_mut_bundleable().try_bundle(other_name, other);
        if let Ok(bool_result) = result {
            self.bundled = self.bundled || bool_result;
        }
        return result;
    }

    fn did_start(&self) -> bool { self.begin_result.is_some() }
}

pub trait BundleableControllerWrapper: ControllerWrapper + BundleableWrapper {
    fn into_controller(self: Box<Self>) -> Box<dyn ControllerWrapper>;
    fn as_ref_controller(&self) -> &dyn ControllerWrapper;
    fn as_mut_controller(&mut self) -> &mut dyn ControllerWrapper;
    fn as_ref_bundleable(&self) -> &dyn BundleableWrapper;
    fn as_mut_bundleable(&mut self) -> &mut dyn BundleableWrapper;
}

// TODO: requires static lifetime...?
impl<T: ControllerWrapper + BundleableWrapper> BundleableControllerWrapper for T where T: Sized + 'static {
    fn into_controller(self: Box<Self>) -> Box<dyn ControllerWrapper> { self }
    fn as_ref_controller(&self) -> &dyn ControllerWrapper { self }
    fn as_mut_controller(&mut self) -> &mut dyn ControllerWrapper { self }
    fn as_ref_bundleable(&self) -> &dyn BundleableWrapper { self }
    fn as_mut_bundleable(&mut self) -> &mut dyn BundleableWrapper { self }
}