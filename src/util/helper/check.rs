use crate::modules::check;
use crate::modules::object::{Arguments, Paths, Configuration, TimeEntry, TimeFrame};
use crate::modules::check::CheckModule;
use crate::modules::traits::Check;
use crate::try_option;

use serde_json::Value;

pub fn init<'a>(args: &Arguments, paths: &'a Paths, config: &Configuration, check_config: &'a Option<Value>) -> Result<Option<CheckModule<'a>>,String> {
    if check_config.is_some() {
        let check_type = try_option!(check_config.as_ref().unwrap().get("type"), "Check config contains no field 'type'");
        let module_paths = paths.for_module(config.name.as_str(), "check", &config.original_path, &config.store_path);

        let mut module = check::get_module(try_option!(check_type.as_str(), "Expected controller type as string"))?;
        module.init(config.name.as_str(), check_config.as_ref().unwrap(), module_paths, args.dry_run, args.no_docker)?;

        return Ok(Some(module));
    } else {
        return Ok(None);
    }
}

pub fn run(module: &Option<CheckModule>, frame: &TimeFrame, last: &Option<&TimeEntry>) -> Result<bool,String> {
    if module.is_some() {
        let result = module.as_ref().unwrap().check(frame, last)?;
        if result {
            debug!("");
        } else {
            debug!("");
        }
        return Ok(result);
    }

    return Ok(true);
}

pub fn update(module: &Option<CheckModule>, frame: &TimeFrame, last: &Option<&TimeEntry>) -> Result<(),String> {
    if module.is_some() {
        module.as_ref().unwrap().update(frame, last)?;
    }

    return Ok(());
}

pub fn clear(module: &mut Option<CheckModule>) -> Result<(),String> {
    if module.is_some() {
        module.as_mut().unwrap().clear()?;
    }

    return Ok(());
}