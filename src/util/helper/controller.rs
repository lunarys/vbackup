use crate::modules::controller;
use crate::modules::object::{Arguments, Paths, Configuration};
use crate::modules::controller::ControllerModule;
use crate::modules::traits::Controller;
use crate::try_option;

use serde_json::Value;

pub fn init(args: &Arguments, paths: &Paths, config: &Configuration, controller_config: &Option<Value>) -> Result<Option<ControllerModule>,String> {
    if controller_config.is_some() {
        let controller_type = try_option!(controller_config.as_ref().unwrap().get("type"), "Controller config contains no field 'type'");
        let module_paths = paths.for_module(config.name.as_str(), "controller", &config.original_path, &config.store_path, &config.savedata_in_store);

        let mut module = controller::get_module(try_option!(controller_type.as_str(), "Expected controller type as string"))?;
        module.init(config.name.as_str(), controller_config.as_ref().unwrap(), module_paths, args.dry_run, args.no_docker)?;

        return Ok(Some(module));
    } else {
        return Ok(None);
    }
}

pub fn start(module: &Option<ControllerModule>) -> Result<bool,String> {
    if module.is_some() {
        let result = module.as_ref().unwrap().begin()?;
        if result {
            debug!("");
        } else {
            debug!("");
        }
        return Ok(result);
    }

    // No controller means sync can be started
    return Ok(true);
}

pub fn end(module: &Option<ControllerModule>) -> Result<bool,String> {
    if module.is_some() {
        let result = module.as_ref().unwrap().end()?;
        if result {
            debug!("");
        } else {
            debug!("");
        }
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