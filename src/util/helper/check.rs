use crate::modules::check::{CheckModule, CheckWrapper};
use crate::modules::check::Reference;
use crate::util::objects::time::{ExecutionTiming};
use crate::util::objects::paths::{Paths,ModulePaths};
use crate::util::objects::configuration::Configuration;
use crate::try_option;
use crate::Arguments;

use serde_json::Value;
use std::rc::Rc;

pub fn init(args: &Rc<Arguments>, paths: &Rc<Paths>, config: &Configuration, check_config: &Option<Value>, reference: Reference) -> Result<Option<CheckModule>,String> {
    return if check_config.is_some() {
        let check_type = try_option!(check_config.as_ref().unwrap().get("type"), "Check config contains no field 'type'");
        let module_paths = ModulePaths::for_check_module(paths, "check", &config, reference);

        let mut module = CheckModule::new(
            try_option!(check_type.as_str(), "Expected check type as string"),
            config.name.as_str(),
            check_config.as_ref().unwrap(),
            module_paths,
            args
        )?;
        module.init()?;

        Ok(Some(module))
    } else {
        Ok(None)
    }
}

pub fn run(module: &mut Option<CheckModule>, timing: &ExecutionTiming) -> Result<bool,String> {
    if let Some(check_module) = module {
        let result = check_module.check(timing)?;
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
    if let Some(check_module) = module {
        check_module.update(timing)?;
    }

    return Ok(());
}

pub fn clear(module: &mut Option<CheckModule>) -> Result<(),String> {
    if let Some(check_module) = module {
        check_module.clear()?;
    }

    return Ok(());
}