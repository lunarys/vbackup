use crate::modules::traits::Controller;
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
    bind: Option<Box<ControllerModule>>,
    init_result: Result<(),String>,
    begin_result: Option<Result<bool,String>>
}

impl ControllerBundle {
    pub fn new(args: &Arguments, paths: &Rc<Paths>, config: &Configuration, controller_bundle: &SyncControllerBundle) -> Result<ControllerBundle,String> {
        let controller_result = controller_helper::init(args, paths, config, &Some(&controller_bundle.controller));
        let (controller_option,init_result) = match controller_result {
            Ok(result) => (result, Ok(())),
            Err(err) => (None, Err(err))
        };

        if let Some(controller) = controller_option {
            return Ok(ControllerBundle {
                bind: Some(Box::new(controller)),
                init_result,
                begin_result: None
            });
        } else {
            // log error and return
            let result = Err(String::from("Received controller bundle without controller configuration"));
            log_error!(result.as_ref());
            return result;
        }
    }

    pub fn wrap(self) -> ControllerModule {
        return ControllerModule::Bundle(self);
    }

    pub fn done(&mut self) -> Result<(),String> {
        // TODO: Handle results
        if let Some(bound) = self.bind.as_mut() {
            let end_result = bound.end();
            let clear_result = bound.clear();
            return end_result.and(clear_result);
        } else {
            return Err(String::from("Called done unbound controller bundle"));
        }
    }
}

impl Controller for ControllerBundle {
    fn init(&mut self, name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<(), String> {
        return self.init_result.clone();
    }

    fn begin(&mut self) -> Result<bool, String> {
        if let Some(begin_result) = self.begin_result.as_ref() {
            return begin_result.clone();
        } else {
            // TODO: begin here
            unimplemented!();
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