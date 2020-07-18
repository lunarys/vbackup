use crate::modules::object::*;
use crate::modules::backup::BackupModule;
use crate::modules;
use crate::modules::check::Reference;
use crate::modules::traits::Backup;
use crate::util::helper::{controller as controller_helper,check as check_helper};
use crate::util::io::savefile::{time_format,write_savedata};

use crate::{try_option, dry_run,log_error};

use chrono::{Local, DateTime, Duration};
use std::ops::Add;

pub fn backup(args: &Arguments, paths: ModulePaths, config: &Configuration, backup_config: BackupConfiguration, savedata: &mut SaveData, timeframes: &TimeFrames) -> Result<bool,String> {
    // Get the backup module that should be used
    let mut module: BackupModule = modules::backup::get_module(backup_config.backup_type.as_str())?;

    // Prepare current timestamp (for consistency) and queue of timeframes for backup
    let current_time : DateTime<Local> = chrono::Local::now();
    let mut queue_refs: Vec<&TimeFrameReference> = vec![];
    let mut queue_frame_entry: Vec<(&TimeFrame, Option<TimeEntry>)> = vec![];

    // Init additional check
    let mut check_module = if !args.force {
        check_helper::init(&args, &paths.base_paths, &config, &backup_config.check, Reference::Backup)?
    } else {
        // No additional check is required if forced run (would be disregarded anyways)
        None
    };

    // Log that this run is forced
    if args.force {
        // Run is forced
        info!("Forcing run of '{}' backup", config.name.as_str());
    }

    // Fill queue with timeframes to run backup for
    for timeframe_ref in &backup_config.timeframes {

        // if amount of saves is zero just skip further checks
        if timeframe_ref.amount.eq(&usize::min_value()) {
            // min_value is 0
            warn!("Amount of saves in timeframe '{}' for '{}' backup is zero, no backup will be created", &timeframe_ref.frame, config.name.as_str());
            continue;
        }

        // Parse time frame data
        let timeframe_opt = timeframes.get(&timeframe_ref.frame);
        let timeframe = if timeframe_opt.is_some() {
            timeframe_opt.unwrap()
        } else {
            error!("Referenced timeframe '{}' for '{}' backup does not exist", &timeframe_ref.frame, config.name.as_str());
            continue;
        };

        // Get last backup (option as there might not be a last one)
        let last_backup_option = savedata.lastsave.remove_entry(&timeframe.identifier);

        // Only actually do check if the run is not forced
        let mut do_backup = true;
        if !args.force {
            let last_backup = if last_backup_option.is_some() {
                let (_, tmp) = last_backup_option.as_ref().unwrap();
                Some(tmp)
            } else {
                None
            };

            // Try to compare timings to the last run
            if last_backup.is_some() {

                // Compare elapsed time since last backup and the configured timeframe
                if last_backup.unwrap().timestamp + timeframe.interval < current_time.timestamp() {
                    // do sync
                    debug!("Backup for '{}' is required in timeframe '{}' considering the interval only", config.name.as_str(), timeframe_ref.frame.as_str());
                } else {
                    // don not sync
                    info!("Backup for '{}' is not executed in timeframe '{}' due to the interval", config.name.as_str(), timeframe_ref.frame.as_str());
                    do_backup = false;
                }
            } else {

                // Probably the first backup in this timeframe, just do it
                info!("This is probably the first backup run in timeframe '{}' for '{}', interval check is skipped", timeframe_ref.frame.as_str(), config.name.as_str());
            }

            // If this point of the loop is reached, only additional check is left to run
            // The helper would check if there is a check module, but this is for more consistent log output
            if do_backup && check_module.is_some() {
                if check_helper::run(&check_module, &current_time, timeframe, &last_backup)? {
                    // Do backup
                    debug!("Backup for '{}' is required in timeframe '{}' considering the additional check", config.name.as_str(), timeframe_ref.frame.as_str());
                } else {
                    // Don't run backup
                    info!("Backup for '{}' is not executed in timeframe '{}' due to the additional check", config.name.as_str(), timeframe_ref.frame.as_str());
                    do_backup = false;
                }
            }
        } else {
            debug!("Run in timeframe '{}' is forced", timeframe_ref.frame.as_str())
        }

        if do_backup {
            queue_refs.push(timeframe_ref);
            queue_frame_entry.push((timeframe, last_backup_option.map(|(_,entry)| entry)));
        } else {
            // Reinsert into the map if not further processed
            if let Some((key, value)) = last_backup_option {
                savedata.lastsave.insert(key, value);
            }
        }
    }

    // Is any backup required?
    if queue_refs.is_empty() {
        // No backup at all is required (for this configuration)
        return Ok(false);
    }

    // Print this here to not have it over and over from the loop
    if check_module.is_none() && !args.force {
        debug!("There is no additional check for the backup of '{}', only using the interval checks", config.name.as_str());
    }

    // For traceability in the log
    info!("Executing backup for '{}'", config.name.as_str());

    // Save value from paths for later
    let save_data_path = paths.save_data.clone();

    // Set up backup module now
    trace!("Invoking backup module");
    module.init(&config.name, &backup_config.config, paths, args)?;

    // Do backups (all timeframes at once to enable optimizations)
    let backup_result = module.backup(&current_time, &queue_refs);
    trace!("Backup module is done");

    // Update internal state of check module and savedata
    if backup_result.is_ok() {

        // Update needs to be done for all active timeframes
        for (frame, entry_opt) in queue_frame_entry {
            // Update check state
            trace!("Invoking state update for additional check in timeframe '{}'", frame.identifier.as_str());
            if let Err(err) = check_helper::update(&check_module, &current_time, frame, &entry_opt.as_ref()) {
                error!("State update for additional check in timeframe '{}' failed ({})", frame.identifier.as_str(), err);
            }

            // Estimate the time of the next required backup (only considering timeframes)
            let next_save = current_time.clone().add(Duration::seconds(frame.interval));

            // Update savedata
            savedata.lastsave.insert(frame.identifier.clone(), TimeEntry {
                timestamp: current_time.timestamp(),
                date: Some(time_format(&current_time))
            });

            savedata.nextsave.insert(frame.identifier.clone(), TimeEntry {
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
            trace!("Writing new savedata to '{}'", save_data_path.as_str());
            if let Err(err) = write_savedata(save_data_path.as_str(), savedata) {
                error!("Could not update savedata for '{}' backup ({})", config.name.as_str(), err);
            }
        } else {
            dry_run!(format!("Updating savedata: {}", save_data_path.as_str()));
        }
    }

    // Check can be freed as it is not required anymore
    if let Err(err) = check_helper::clear(&mut check_module) {
        error!("Could not clear the check module: {}", err);
    }

    // Free backup module now
    if let Err(err) = module.clear() {
        error!("Could not clear backup module: {}", err);
    }

    return backup_result.map(|_| true);
}