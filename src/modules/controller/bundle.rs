use crate::modules::traits::{Controller, Bundleable};
use crate::modules::controller::ControllerModule;
use crate::processing::scheduler::SyncControllerBundle;
use crate::util::objects::configuration::Configuration;
use crate::util::helper::{controller as controller_helper};
use crate::util::objects::paths::{Paths, ModulePaths};
use crate::Arguments;

use crate::{log_error};

use serde_json::Value;
use std::rc::Rc;

pub struct ControllerBundle {
    bind: Box<ControllerModule>,
    init_result: Result<(),String>,
    begin_result: Option<Result<bool,String>>
}

impl ControllerBundle {
    pub fn new(mut main_controller: ControllerModule, other_controllers: Vec<ControllerModule>) -> Result<ControllerBundle,String> {
        let init_result = main_controller.init_bundle(other_controllers);

        return Ok(ControllerBundle {
            bind: Box::new(main_controller),
            init_result,
            begin_result: None
        });
    }

    pub fn wrap(self) -> ControllerModule {
        return ControllerModule::Bundle(self);
    }

    pub fn done(&mut self) -> Result<(),String> {
        // TODO: Handle results
        let end_result = self.bind.end();
        let clear_result = self.bind.clear();
        return end_result.and(clear_result);
    }
}

impl Controller for ControllerBundle {
    fn init(&mut self, _name: &str, _config_json: &Value, _paths: ModulePaths, _args: &Arguments) -> Result<(), String> {
        return self.init_result.clone();
    }

    fn begin(&mut self) -> Result<bool, String> {
        if let Some(result) = self.begin_result.as_ref() {
            return result.clone();
        } else {
            let result = self.bind.begin();
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
}