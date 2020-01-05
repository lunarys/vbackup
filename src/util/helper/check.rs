use crate::modules::check;
use crate::modules::object::{Arguments, Paths, Configuration, TimeEntry};
use crate::modules::check::CheckModule;
use crate::modules::traits::Check;
use crate::try_option;

use serde_json::Value;

pub fn init(args: &Arguments, paths: &Paths, config: &Configuration, check_config: &Option<Value>, last: &Option<&TimeEntry>) -> Result<Option<CheckModule>,String> {
    if check_config.is_some() {
        let check_type = try_option!(check_config.as_ref().unwrap().get("type"), "Check config contains no field 'type'");
        let module_paths = paths.for_module(config.name.as_str(), "check", &config.original_path, &config.store_path);

        let mut module = check::get_module(try_option!(check_type.as_str(), "Expected controller type as string"))?;
        module.init(config.name.as_str(), check_config.as_ref().unwrap(), last, module_paths, args.dry_run, args.no_docker)?;

        return Ok(Some(module));
    } else {
        return Ok(None);
    }
}

pub fn run(module: &Option<CheckModule>) -> Result<bool,String> {
    if module.is_some() {
        let result = module.as_ref().unwrap().check()?;
        if result {
            debug!("");
        } else {
            debug!("");
        }
        return Ok(result);
    }

    return Ok(true);
}

pub fn update(module: &Option<CheckModule>) -> Result<(),String> {
    if module.is_some() {
        module.as_ref().unwrap().update()?;
    }

    return Ok(());
}

pub fn clear(module: &mut Option<CheckModule>) -> Result<(),String> {
    if module.is_some() {
        module.as_mut().unwrap().clear()?;
    }

    return Ok(());
}