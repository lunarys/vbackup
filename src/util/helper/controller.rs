use crate::modules::controller;
use crate::modules::controller::ControllerModule;
use crate::modules::traits::Controller;
use crate::try_option;
use crate::Arguments;
use crate::util::objects::paths::{Paths, ModulePaths};
use crate::util::objects::configuration::Configuration;

use serde_json::Value;
use std::rc::Rc;

pub fn init(args: &Arguments, paths: &Rc<Paths>, config: &Configuration, controller_config: &Option<&Value>) -> Result<Option<ControllerModule>,String> {
    if controller_config.is_some() {
        let controller_type = try_option!(controller_config.unwrap().get("type"), "Controller config contains no field 'type'");
        let module_paths = ModulePaths::for_sync_module(paths, "controller", &config);

        let mut module = controller::get_module(try_option!(controller_type.as_str(), "Expected controller type as string"))?;
        module.init(config.name.as_str(), controller_config.unwrap(), module_paths, args)?;

        return Ok(Some(module));
    } else {
        return Ok(None);
    }
}

pub fn start(module: &mut Option<ControllerModule>) -> Result<bool,String> {
    if module.is_some() {
        let result = module.as_mut().unwrap().begin()?;
        /*if result {
            debug!("");
        } else {
            debug!("");
        }*/
        return Ok(result);
    }

    // No controller means sync can be started
    return Ok(true);
}

pub fn end(module: &mut Option<ControllerModule>) -> Result<bool,String> {
    if module.is_some() {
        let result = module.as_mut().unwrap().end()?;
        /*if result {
            debug!("");
        } else {
            debug!("");
        }*/
        return Ok(result);
    }

    return Ok(true);
}

pub fn clear(module: &mut Option<ControllerModule>) -> Result<(),String> {
    if module.is_some() {
        module.as_mut().unwrap().clear()?;
    }

    return Ok(());
}