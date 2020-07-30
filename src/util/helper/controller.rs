use crate::modules::controller;
use crate::modules::controller::ControllerModule;
use crate::modules::traits::{Controller, Bundleable};
use crate::try_option;
use crate::Arguments;
use crate::util::objects::paths::{Paths, ModulePaths};
use crate::util::objects::configuration::Configuration;

use serde_json::Value;
use std::rc::Rc;

pub fn init(args: &Arguments, paths: &Rc<Paths>, config: &Configuration, controller_config: &Option<&Value>) -> Result<Option<ControllerModule>,String> {
    if controller_config.is_some() {
        let controller_type = try_option!(controller_config.unwrap().get("type"), "Controller config contains no field 'type'");

        let mut module = controller::get_module(try_option!(controller_type.as_str(), "Expected controller type as string"))?;
        if module.can_bundle() {
            // Bundleable requires an additional step for pre_init
            module.pre_init(config.name.as_str(), controller_config.unwrap(), paths, args)?;
        } else {
            // If not Bundleable, it can be initiated directly
            let module_paths = ModulePaths::for_sync_module(paths, "controller", &config);
            module.init(config.name.as_str(), controller_config.unwrap(), module_paths, args)?;
        }

        return Ok(Some(module));
    } else {
        return Ok(None);
    }
}

pub fn start(module: &mut Option<&ControllerModule>) -> Result<bool,String> {
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

pub fn end(module: &mut Option<&ControllerModule>) -> Result<bool,String> {
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

pub fn clear(module: &mut Option<&ControllerModule>) -> Result<(),String> {
    if module.is_some() {
        module.as_mut().unwrap().clear()?;
    }

    return Ok(());
}