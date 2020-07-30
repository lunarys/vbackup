use crate::modules::check;
use crate::modules::check::CheckModule;
use crate::modules::traits::Check;
use crate::modules::check::Reference;
use crate::util::objects::time::{TimeEntry, TimeFrame, ExecutionTiming};
use crate::util::objects::paths::{Paths,ModulePaths};
use crate::util::objects::configuration::Configuration;
use crate::try_option;
use crate::Arguments;

use serde_json::Value;
use chrono::{DateTime, Local};
use std::rc::Rc;

pub fn init(args: &Arguments, paths: &Rc<Paths>, config: &Configuration, check_config: &Option<Value>, reference: Reference) -> Result<Option<CheckModule>,String> {
    if check_config.is_some() {
        let check_type = try_option!(check_config.as_ref().unwrap().get("type"), "Check config contains no field 'type'");
        let module_paths = ModulePaths::for_check_module(paths, "check", &config, reference);

        let mut module = check::get_module(try_option!(check_type.as_str(), "Expected check type as string"))?;
        module.init(config.name.as_str(), check_config.as_ref().unwrap(), module_paths, args)?;

        return Ok(Some(module));
    } else {
        return Ok(None);
    }
}

pub fn run(module: &Option<CheckModule>, timing: &ExecutionTiming) -> Result<bool,String> {
    if module.is_some() {
        let result = module.as_ref().unwrap().check(timing)?;
        /*if result {
            debug!("TODO: check_helper debug");
        } else {
            debug!("TODO: check_helper debug");
        }*/
        return Ok(result);
    }

    return Ok(true);
}

pub fn update(module: &mut Option<CheckModule>, timing: &ExecutionTiming) -> Result<(),String> {
    if module.is_some() {
        module.as_mut().unwrap().update(timing)?;
    }

    return Ok(());
}

pub fn clear(module: &mut Option<CheckModule>) -> Result<(),String> {
    if module.is_some() {
        module.as_mut().unwrap().clear()?;
    }

    return Ok(());
}