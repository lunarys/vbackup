use crate::modules::object::*;
use crate::modules::sync::SyncModule;
use crate::modules::check::Reference;
use crate::modules;
use crate::modules::traits::Sync;
use crate::util::helper::{controller as controller_helper,check as check_helper};
use crate::util::io::savefile::{time_format, write_savedata};

use crate::{try_option, dry_run,log_error};

use chrono::{DateTime, Local};
use std::borrow::Borrow;
use crate::util::objects::time::{SaveData, TimeFrames, TimeFrame, TimeEntry};

pub fn sync(args: &Arguments, paths: ModulePaths, config: &Configuration, sync_config: SyncConfiguration, savedata: &mut SaveData, timeframes: &TimeFrames) -> Result<bool,String> {
    // Get the sync module that should be used
    let mut module: SyncModule = modules::sync::get_module(sync_config.sync_type.as_str())?;

    // Prepare current timestamp and get timestamp of last backup + referenced timeframe
    let current_time: DateTime<Local> = chrono::Local::now();
    let last_sync_opt = savedata.lastsync.get(&sync_config.interval.frame);
    let timeframe: &TimeFrame = try_option!(timeframes.get(&sync_config.interval.frame), "Referenced timeframe for sync does not exist");

    // Module paths are moved, keep a reference to the base paths
    let base_paths = paths.base_paths.clone();
    let mut check_module = if !args.force {
        check_helper::init(&args, &base_paths, &config, &sync_config.check, Reference::Sync)?
    } else {
        // No additional check is required if forced run
        None
    };

    // If the run is forced no other checks are required
    if !args.force {

        // Compare to last sync timestamp (if it exists)
        if let Some(last_sync) = last_sync_opt {

            // Compare elapsed time since last sync and the configured timeframe
            if last_sync.timestamp + timeframe.interval >= current_time.timestamp() {
                // sync not necessary
                info!("Sync for '{}' is not executed due to the constraints of timeframe '{}'", config.name.as_str(), timeframe.identifier.as_str());
                return Ok(false);
            }

            // Check with last backup time
            let backup_after_sync = savedata.lastsave.is_empty() || savedata.lastsave.values().any(|backup| backup.timestamp > last_sync.timestamp );
            if !backup_after_sync {
                info!("Sync for '{}' is not executed as there is no new backup since the last sync", config.name.as_str());
                return Ok(false);
            }

            // do sync
            debug!("Sync for '{}' is required considering the timeframe '{}' only", config.name.as_str(), timeframe.identifier.as_str());
        } else {

            // This is probably the first sync, so just do it
            info!("This is probably the first sync run for '{}', interval check is skipped", config.name.as_str());
        }

        // Run additional check
        if check_module.is_some() {
            if check_helper::run(&check_module, &current_time, timeframe, &last_sync_opt)? {
                // Do sync
                debug!("Sync for '{}' is required considering the additional check", config.name.as_str());
            } else {
                // Do not run sync
                debug!("");
                return Ok(false);
            }
        } else {
            debug!("There is no additional check for the sync of '{}', only using the interval check", config.name.as_str());
        }

        // If we did not leave the function by now sync is necessary
        info!("Executing sync for '{}'", config.name.as_str());
    } else {
        // Run is forced
        info!("Forcing sync for '{}'", config.name.as_str())
    }

    // Save path is still required after move, make a copy
    let save_data_path = paths.save_data.clone();

    // Initialize sync module
    module.init(&config.name, &sync_config.config, paths, args)?;

    // Set up controller (if configured)
    let mut controller_module = controller_helper::init(&args, &base_paths, &config, &sync_config.controller.as_ref())?;

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
        trace!("Invoking state update for additional check in timeframe '{}'", timeframe.identifier.as_str());
        if let Err(err) = check_helper::update(&mut check_module, &current_time, timeframe, &last_sync_opt) {
            error!("State update for additional check in timeframe '{}' failed ({})", timeframe.identifier.as_str(), err);
        }

        // Update save data
        savedata.lastsync.insert(timeframe.identifier.clone(), TimeEntry {
            timestamp: current_time.timestamp(),
            date: Some(time_format(&current_time))
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
            trace!("Writing new savedata to '{}'", save_data_path.as_str());
            if let Err(err) = write_savedata(save_data_path.as_str(), savedata) {
                error!("Could not update savedata for '{}' sync ({})", config.name.as_str(), err);
            }
        } else {
            dry_run!(format!("Updating savedata: {}", save_data_path.as_str()));
        }
    }

    // Check can be freed as it is not required anymore
    if let Err(err) = check_helper::clear(&mut check_module) {
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