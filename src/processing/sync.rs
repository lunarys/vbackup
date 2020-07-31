use crate::modules::sync::SyncModule;
use crate::modules;
use crate::modules::traits::Sync;
use crate::modules::controller::ControllerModule;
use crate::util::helper::{controller as controller_helper,check as check_helper};
use crate::util::io::savefile::{time_format};
use crate::util::objects::time::{SaveData, TimeEntry};
use crate::processing::preprocessor::SyncUnit;
use crate::Arguments;

use crate::{dry_run};

pub fn sync(args: &Arguments, unit: &mut SyncUnit, savedata: &mut SaveData, controller_override: Option<&mut ControllerModule>) -> Result<bool,String> {
    // Get the sync module that should be used
    let mut controller_module = controller_override.or(unit.controller.as_mut());
    let mut module: SyncModule = modules::sync::get_module(unit.sync_config.sync_type.as_str())?;

    info!("Executing sync for '{}'", unit.config.name.as_str());

    // Initialize sync module
    // TODO: clone
    module.init(&unit.config.name, &unit.sync_config.config, unit.module_paths.clone(), args)?;

    // Run controller (if there is one)
    if controller_module.is_some() {
        trace!("Invoking remote device controller");
        if controller_helper::start(&mut controller_module)? {
            // There is no controller or device is ready for sync
            info!("Remote device is now available");
        } else {
            // Device did not start before timeout or is not available
            warn!("Remote device is not available, aborting sync");
            return Ok(false);
        }
    }

    // Run sync
    trace!("Invoking sync module");
    let sync_result = module.sync();

    // Check result of sync and act accordingly
    if sync_result.is_ok() {
        trace!("Sync module is done");

        // Update internal state of check
        trace!("Invoking state update for additional check in timeframe '{}'", unit.timeframe.time_frame.identifier.as_str());
        if let Err(err) = check_helper::update(&mut unit.check, &unit.timeframe) {
            error!("State update for additional check in timeframe '{}' failed ({})", unit.timeframe.time_frame.identifier.as_str(), err);
        }

        // Update save data
        savedata.lastsync.insert(unit.timeframe.time_frame.identifier.clone(), TimeEntry {
            timestamp: unit.timeframe.execution_time.timestamp(),
            date: Some(time_format(&unit.timeframe.execution_time))
        });
    } else {
        trace!("Sync failed, cleaning up");
    }

    // Run controller end (result is irrelevant here)
    if let Err(err) = controller_helper::end(&mut controller_module) {
        error!("Stopping the remote device after use failed: {}", err);
    }

    // Write savedata update only if sync was successful
    if sync_result.is_ok() {
        if !args.dry_run {
            if let Err(err) = savedata.write() {
                error!("Could not update savedata for '{}' sync ({})", unit.config.name.as_str(), err);
            }
        } else {
            dry_run!(format!("Updating savedata: {}", savedata.path.as_str()));
        }
    }

    // Check can be freed as it is not required anymore
    if let Err(err) = check_helper::clear(&mut unit.check) {
        error!("Could not clear the check module: {}", err);
    }

    // Controller can be freed as it is not required anymore
    if let Err(err) = controller_helper::clear(&mut controller_module) {
        error!("Could not clear the controller module: {}", err);
    }

    // Free sync module
    if let Err(err) = module.clear() {
        error!("Could no clear sync module: {}", err);
    }

    // Return Ok(true) for sync was executed or Err(error) for failed sync
    return sync_result.map(|_| true);
}