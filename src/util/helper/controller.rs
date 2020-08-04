use crate::modules::controller::{ControllerModule, ControllerRelay};

pub fn init(module: &mut Option<&mut ControllerModule>) -> Result<(),String> {
    if let Some(module_mut) = module {
        return module_mut.init();
    }

    return Ok(());
}

pub fn start(module: &mut Option<&mut ControllerModule>) -> Result<bool,String> {
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

pub fn end(module: &mut Option<&mut ControllerModule>) -> Result<bool,String> {
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

pub fn clear(module: &mut Option<&mut ControllerModule>) -> Result<(),String> {
    if module.is_some() {
        module.as_mut().unwrap().clear()?;
    }

    return Ok(());
}