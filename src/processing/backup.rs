use crate::modules::backup::BackupModule;
use crate::modules;
use crate::modules::check::Reference;
use crate::modules::traits::Backup;
use crate::util::helper::{controller as controller_helper,check as check_helper};
use crate::util::io::savefile::{time_format,write_savedata};
use crate::util::objects::time::{SaveData, TimeFrames, TimeFrameReference, TimeFrame, TimeEntry};
use crate::util::objects::paths::{Paths, ModulePaths};
use crate::util::objects::configuration::{Configuration,BackupConfiguration,SyncConfiguration};
use crate::processing::preprocessor::BackupUnit;
use crate::Arguments;

use crate::{try_option, dry_run,log_error};

use chrono::{Local, DateTime, Duration};
use std::ops::Add;
use std::rc::Rc;

pub fn backup(args: &Arguments, unit: &mut BackupUnit, savedata: &mut SaveData) -> Result<bool,String> {
    // Get the backup module that should be used
    let mut module: BackupModule = modules::backup::get_module(unit.backup_config.backup_type.as_str())?;

    // Is any backup required?
    if unit.timeframes.is_empty() {
        // No backup is required (for this configuration)
        return Ok(false);
    }

    // For traceability in the log
    info!("Executing backup for '{}'", unit.config.name.as_str());

    // Set up backup module now
    trace!("Invoking backup module");
    // TODO: clone
    module.init(&unit.config.name, &unit.backup_config.config, unit.module_paths.clone(), args)?;

    // Do backups (all timeframes at once to enable optimizations)
    let backup_result = module.backup(&unit.timeframes);
    trace!("Backup module is done");

    // Update internal state of check module and savedata
    if backup_result.is_ok() {

        // Update needs to be done for all active timeframes
        for timing in &unit.timeframes {
            // Update check state
            trace!("Invoking state update for additional check in timeframe '{}'", timing.time_frame.identifier.as_str());
            if let Err(err) = check_helper::update(&mut unit.check, timing) {
                error!("State update for additional check in timeframe '{}' failed ({})", timing.time_frame.identifier.as_str(), err);
            }

            // Estimate the time of the next required backup (only considering timeframes)
            let next_save = timing.execution_time.clone().add(Duration::seconds(timing.time_frame.interval));

            // Update savedata
            savedata.lastsave.insert(timing.time_frame.identifier.clone(), TimeEntry {
                timestamp: timing.execution_time.timestamp(),
                date: Some(time_format(&timing.execution_time))
            });

            savedata.nextsave.insert(timing.time_frame.identifier.clone(), TimeEntry {
                timestamp: next_save.timestamp(),
                date: Some(time_format(&next_save))
            });
        }
    } else {
        error!("Backup failed, cleaning up");
    }

    // Write savedata update only if backup was successful
    if backup_result.is_ok() {
        if !args.dry_run {
            if let Err(err) = savedata.write() {
                error!("Could not update savedata for '{}' backup ({})", unit.config.name.as_str(), err);
            }
        } else {
            dry_run!(format!("Updating savedata: {}", savedata.path.as_str()));
        }
    }

    // Check can be freed as it is not required anymore
    if let Err(err) = check_helper::clear(&mut unit.check) {
        error!("Could not clear the check module: {}", err);
    }

    // Free backup module now
    if let Err(err) = module.clear() {
        error!("Could not clear backup module: {}", err);
    }

    return backup_result.map(|_| true);
}