use crate::modules::object::*;
use crate::util::io::{file,json};
use crate::util::helper::{controller as controller_helper,check as check_helper};
use crate::modules;
use crate::modules::traits::{Sync, Backup, Reporting};
use crate::modules::sync::SyncModule;
use crate::modules::backup::BackupModule;
use crate::modules::check::Reference;
use crate::modules::reporting::ReportingModule;

use crate::{try_option, dry_run,log_error};

use std::path::Path;
use serde_json::Value;
use std::collections::HashMap;
use std::ops::Add;
use core::borrow::Borrow;
use chrono::{DateTime, Local, Duration};

pub fn main(args: Arguments) -> Result<(),String> {
    let base_paths = json::from_file::<PathBase>(Path::new(args.base_config.as_str()))?;
    let paths = Paths::from(base_paths);

    file::create_dir_if_missing(paths.save_dir.as_str(), true)?;
    file::create_dir_if_missing(paths.tmp_dir.as_str(), true)?;

    let timeframes = json::from_file::<TimeFrames>(Path::new(&paths.timeframes_file))?;
    let mut reporter = match args.operation.as_str() {
        "run" | "backup" | "sync" => {
            let reporter_config_opt = json::from_file_checked::<Value>(Path::new(paths.reporting_file.as_str()))?;
            if let Some(reporter_config) = reporter_config_opt {
                let mut r = ReportingModule::new_combined();
                r.init(&reporter_config, &paths, args.dry_run, args.no_docker)?;
                r
            } else {
                ReportingModule::new_empty()
            }
        },
        _ => ReportingModule::new_empty()
    };

    // Only actually does something if run, backup or sync
    log_error!(reporter.report(None, args.operation.as_str()));

    let result = match args.operation.as_str() {
        "run" => {
            let backup_result = backup_wrapper(&args, &paths, &timeframes, &reporter);
            if let Ok((original_size, backup_size)) = backup_result.as_ref() {
                log_error!(reporter.report(Some(&["size", "original"]), original_size.to_string().as_str()));
                log_error!(reporter.report(Some(&["size", "backup"]), backup_size.to_string().as_str()));
            }

            // If backup failed do not try to sync, there is probably nothing to sync
            let sync_result = backup_result.and(sync_wrapper(&args, &paths, &timeframes, &reporter));
            if let Ok(sync_size) = sync_result.as_ref() {
                log_error!(reporter.report(Some(&["size", "sync"]), sync_size.to_string().as_str()));
            }

            sync_result.map(|_| ())
        },
        "backup" => {
            let result = backup_wrapper(&args, &paths, &timeframes, &reporter);
            if let Ok((original_size, backup_size)) = result.as_ref() {
                log_error!(reporter.report(Some(&["size", "original"]), original_size.to_string().as_str()));
                log_error!(reporter.report(Some(&["size", "backup"]), backup_size.to_string().as_str()));
            }
            result.map(|_| ())
        },
        "save" => {
            let result = sync_wrapper(&args, &paths, &timeframes, &reporter);
            if let Ok(sync_size) = result.as_ref() {
                log_error!(reporter.report(Some(&["size", "sync"]), sync_size.to_string().as_str()));
            }
            result.map(|_| ())
        },
        "list" => list(&args, &paths),
        unknown => {
            let err = format!("Unknown operation: '{}'", unknown);
            //throw!(err);
            Err(err)
        }
    };

    log_error!(reporter.report(None, "done"));
    log_error!(reporter.clear());
    return result;
}

fn backup_wrapper(args: &Arguments, paths: &Paths, timeframes: &TimeFrames, reporter: &ReportingModule) -> Result<(u64,u64),String> {
    // Collect total sizes of the backup
    let mut original_size_acc = 0;
    let mut backup_size_acc = 0;

    // Go through all configurations in the config directory
    for mut config in get_config_list(args, paths)? {

        // Get paths specifically for this module
        let module_paths = paths.for_backup_module("backup", &config);

        // Only do something else if a backup is present in this configuration
        if config.backup.is_some() {

            // Get savedata for this backup
            let savedata_result = get_savedata(module_paths.save_data.as_str());
            let mut savedata = match savedata_result {
                Ok(savedata) => savedata,
                Err(err) => {
                    error!("Could not read savedata for '{}': {}", config.name, err);
                    continue;
                }
            };

            // Announcing the start of this backup
            log_error!(reporter.report(Some(&["backup", config.name.as_str()]), "starting"));

            // Take ownership of config
            let backup_config = config.backup.take().unwrap();

            // Save those paths for later, as the ModulePaths will be moved
            let original_path = module_paths.source.clone();
            let store_path = module_paths.destination.clone();

            // Run the backup and evaluate the result
            let result = backup(args, module_paths, &config, backup_config, &mut savedata, timeframes);
            match result {
                Ok(true) => {
                    info!("Backup for '{}' was successfully executed", config.name.as_str());
                    let report_result =reporter.report(Some(&["backup", config.name.as_str()]), "success");
                    log_error!(report_result);
                },
                Ok(false) => {
                    info!("Backup for '{}' was not executed due to constraints", config.name.as_str());
                    let report_result =reporter.report(Some(&["backup", config.name.as_str()]), "skipped");
                    log_error!(report_result);
                },
                Err(err) => {
                    error!("Backup for '{}' failed: {}", config.name.as_str(), err);
                    let report_result = reporter.report(Some(&["backup", config.name.as_str()]), "failed");
                    log_error!(report_result);
                }
            }

            // Calculate and report the size of the original files
            match file::size(original_path.as_str(), args.no_docker) {
                Ok(curr_size) => {
                    log_error!(reporter.report(Some(&["backup", config.name.as_str(), "size", "original"]), curr_size.to_string().as_str()));
                    original_size_acc += curr_size;
                },
                Err(err) => error!("Could not read size of the original files: {}", err)
            }

            // Calculate and report the size of the backup files
            match file::size(store_path.as_str(), args.no_docker) {
                Ok(curr_size) => {
                    log_error!(reporter.report(Some(&["backup", config.name.as_str(), "size", "backup"]), curr_size.to_string().as_str()));
                    backup_size_acc += curr_size;
                },
                Err(err) => error!("Could not read size of the backup up files: {}", if args.dry_run { "This is likely due to this being a dry-run" } else { err.as_str() })
            }

            // Announce that this backup is done now
            log_error!(reporter.report(Some(&["backup", config.name.as_str()]), "done"));
        } else {
            info!("No backup is configured for '{}'", config.name.as_str());
        }
    }

    // Only fails if some general configuration is not available, failure in single backups is only logged
    Ok((original_size_acc, backup_size_acc))
}

fn backup(args: &Arguments, paths: ModulePaths, config: &Configuration, backup_config: BackupConfiguration, savedata: &mut SaveData, timeframes: &TimeFrames) -> Result<bool,String> {
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
    debug!("Executing backup for '{}'", config.name.as_str());

    // Save value from paths for later
    let save_data_path = paths.save_data.clone();

    // Set up backup module now
    trace!("Invoking backup module");
    module.init(&config.name, &backup_config.config, paths, args.dry_run, args.no_docker)?;

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

fn sync_wrapper(args: &Arguments, paths: &Paths, timeframes: &TimeFrames, reporter: &ReportingModule) -> Result<u64,String> {
    // Collect the total size of synchronized files
    let mut acc_size = 0;

    // Go through all configurations in the config directory
    for mut config in get_config_list(args, paths)? {

        // Get paths specifically for this module
        let module_paths = paths.for_sync_module("sync", &config);

        // Get savedata for this sync
        let savedata_result = get_savedata(module_paths.save_data.as_str());
        let mut savedata = match savedata_result {
            Ok(savedata) => savedata,
            Err(err) => {
                error!("Could not read savedata for '{}': {}", config.name, err);
                continue;
            }
        };

        // Only do something else if a sync is present in this configuration
        if config.sync.is_some() {

            // Announce that this sync is starting
            log_error!(reporter.report(Some(&["sync", config.name.as_str()]), "starting"));

            // Save owned objects of configuration and path
            let sync_config = config.sync.take().unwrap();
            let store_path = module_paths.source.clone();

            // Run the backup and evaluate the result
            let result = sync(args, module_paths, &config, sync_config, &mut savedata, timeframes);
            match result {
                Ok(true) => {
                    info!("Sync for '{}' was successfully executed", config.name.as_str());
                    log_error!(reporter.report(Some(&["sync", config.name.as_str()]), "success"));
                },
                Ok(false) => {
                    info!("Sync for '{}' was not executed due to constraints", config.name.as_str());
                    log_error!(reporter.report(Some(&["sync", config.name.as_str()]), "skipped"));
                },
                Err(err) => {
                    error!("Sync for '{}' failed: {}", config.name.as_str(), err);
                    log_error!(reporter.report(Some(&["sync", config.name.as_str()]), "failed"));
                }
            }

            // Calculate and report size of the synced files
            // TODO: Current implementation just takes the size of the local files...
            match file::size(store_path.as_str(), args.no_docker) {
                Ok(curr_size) => {
                    log_error!(reporter.report(Some(&["sync", config.name.as_str(), "size", "sync"]), curr_size.to_string().as_str()));
                    acc_size += curr_size;
                },
                Err(err) => error!("Could not read size of sync: {}", if args.dry_run { "This is likely due to this being a dry-run" } else { err.as_str() })
            }

            // Announce that this sync is done now
            log_error!(reporter.report(Some(&["sync", config.name.as_str()]), "done"));
        } else {
            info!("No sync is configured for '{}'", config.name.as_str());
        }
    }

    // Only fails if some general configuration is not available, failure in single syncs is only logged
    Ok(acc_size)
}

fn sync(args: &Arguments, paths: ModulePaths, config: &Configuration, sync_config: SyncConfiguration, savedata: &mut SaveData, timeframes: &TimeFrames) -> Result<bool,String> {
    // Get the sync module that should be used
    let mut module: SyncModule = modules::sync::get_module(sync_config.sync_type.as_str())?;

    // Prepare current timestamp and get timestamp of last backup + referenced timeframe
    let current_time: DateTime<Local> = chrono::Local::now();
    let last_sync = savedata.lastsync.get(&sync_config.interval.frame);
    let timeframe: &TimeFrame = try_option!(timeframes.get(&sync_config.interval.frame), "Referenced timeframe for sync does not exist");

    // Save base path for later as it will be moved
    let base_paths = paths.base_paths.borrow();
    let mut check_module = if !args.force {
        check_helper::init(&args, base_paths, &config, &sync_config.check, Reference::Sync)?
    } else {
        // No additional check is required if forced run
        None
    };

    // If the run is forced no other checks are required
    if !args.force {

        // Compare to last sync timestamp (if it exists)
        if last_sync.is_some() {

            // Compare elapsed time since last sync and the configured timeframe
            if last_sync.unwrap().timestamp + timeframe.interval < current_time.timestamp() {
                // do sync
                debug!("Sync for '{}' is required considering the timeframe '{}' only", config.name.as_str(), timeframe.identifier.as_str());
            } else {
                // sync not necessary
                info!("Sync for '{}' is not executed due to the constraints of timeframe '{}'", config.name.as_str(), timeframe.identifier.as_str());
                return Ok(false);
            }
        } else {

            // This is probably the first sync, so just do it
            info!("This is probably the first sync run for '{}', interval check is skipped", config.name.as_str());
        }

        // Run additional check
        if check_module.is_some() {
            if check_helper::run(&check_module, &current_time, timeframe, &last_sync)? {
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
    } else {
        // Run is forced
        info!("Forcing run of '{}' sync", config.name.as_str())
    }

    // If we did not leave the function by now sync is necessary
    debug!("Executing sync for '{}'", config.name.as_str());

    // Save path is still required after move, make a copy
    let save_data_path = paths.save_data.clone();

    // Initialize sync module
    module.init(&config.name, &sync_config.config, paths, args.dry_run, args.no_docker)?;

    // Set up controller (if configured)
    let mut controller_module = controller_helper::init(&args, base_paths, &config, &sync_config.controller)?;

    // Run controller (if there is one)
    if controller_module.is_some() {
        trace!("Invoking remote device controller");
        if controller_helper::start(&controller_module)? {
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
        if let Err(err) = check_helper::update(&check_module, &current_time, timeframe, &last_sync) {
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
    if let Err(err) = controller_helper::end(&controller_module) {
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

pub fn list(args: &Arguments, paths: &Paths) -> Result<(), String> {

    // Helper to output an additional check nicely formatted
    fn print_check(config: &Option<Value>) {
        if let Some(check_config) = config.as_ref() {
            if let Some(check_type) = check_config.get("type") {
                 if let Some(type_str) = check_type.as_str() {
                     println!("     * Additional check of type '{}'", type_str);
                 } else {
                     println!("     * Could not parse type of additional check: Expected string");
                 }
            } else {
                println!("     * Could not parse type of additional check: Expected type field");
            }
        } else {
            println!("     * No additional check configured");
        }
    }

    // Helper to output a controller nicely formatted
    fn print_controller(config: &Option<Value>) {
        if let Some(controller_config) = config.as_ref() {
            if let Some(controller_type) = controller_config.get("type") {
                if let Some(type_str) = controller_type.as_str() {
                    println!("     * Controller of type '{}'", type_str);
                } else {
                    println!("     * Could not parse type of controller: Expected string");
                }
            } else {
                println!("     * Could not parse type of controller: Expected type field");
            }
        } else {
            println!("     * No controller configured");
        }
    }

    // Helper to output a timeframe reference nicely formatted
    fn print_timeframe_ref(frame: &TimeFrameReference, with_amount: bool) {
        print!("       - {}", frame.frame);
        if with_amount {
            print!(", maximal amount: {}", frame.amount);
        }
        println!();
    }

    // Description
    println!("vBackup configurations:");

    // Go through all configurations
    for config in get_config_list(args, paths)? {

        // Get paths for both backup and sync module
        let backup_paths = paths.for_backup_module("backup", &config);
        let sync_paths = paths.for_sync_module("sync", &config);

        // Configuration header
        println!("- Configuration for: {} is {}", config.name.as_str(), if config.disabled {"disabled"} else {"enabled"});

        // Print information on backup if configured
        if let Some(backup_config) = config.backup.as_ref() {
            println!("   + Backup of type '{}' is {}", backup_config.backup_type, if backup_config.disabled {"disabled"} else {"enabled"});

            // Only show more information if not disabled
            if !backup_config.disabled {
                println!("     * Original data path: {}", backup_paths.source);
                println!("     * Backup data path:   {}", backup_paths.destination);

                println!("     * Timeframes for backup:");
                backup_config.timeframes.iter().for_each(|f| print_timeframe_ref(f, true));

                print_check(&backup_config.check)
            }
        } else {
            println!("   x No backup configured");
        }

        // Print information on sync if configured
        if let Some(sync_config) = config.sync.as_ref() {
            println!("   + Sync of type '{}' is {}", sync_config.sync_type, if sync_config.disabled {"disabled"} else {"enabled"});

            // Only show more if not disabled
            if !sync_config.disabled {
                println!("     * Path of synced data: {}", sync_paths.source);

                println!("     * Interval for sync:");
                print_timeframe_ref(&sync_config.interval, false);

                print_check(&sync_config.check);
                print_controller(&sync_config.controller);
            }
        } else {
            println!("   x No sync configured");
        }
    }

    return Ok(());
}

fn get_config_list(args: &Arguments, paths: &Paths) -> Result<Vec<Configuration>, String> {
    // Get directory containing configurations
    let volume_config_path = format!("{}/volumes", &paths.config_dir);

    // Check if a specific one should be outputted
    let files = if args.name.is_some() {

        // Only run this one -> Let the list only contain this item
        let path = format!("{}/{}.json", volume_config_path, args.name.as_ref().unwrap());
        vec![Path::new(&path).to_path_buf()]
    } else {

        // Run all -> Return all the files in the configuration directory
        file::list_in_dir(volume_config_path.as_str())?
    };

    // Load all the configuration files parsed as Configuration
    // TODO: Only logs inaccessible files and then disregards the error
    let configs = files.iter().filter_map(|file_path| {
        let result = json::from_file::<Configuration>(file_path);
        if result.is_ok() {
            Some(result.unwrap())
        } else {
            error!("Could not parse configuration from '{}' ({})", "<filename?>", result.err().unwrap().to_string());
            None
        }
    }).collect();

    return Ok(configs);
}

fn get_savedata(path: &str) -> Result<SaveData, String> {
    // Check if there is a file with savedata
    let savedata = if file::exists(path) {

        // File exists: Read savedata
        json::from_file::<SaveData>(Path::new(path))?
    } else {

        // File does not exist: Create new savedata
        SaveData {
            lastsave: HashMap::new(),
            nextsave: HashMap::new(),
            lastsync: HashMap::new()
        }
    };

    return Ok(savedata);
}

fn write_savedata(path: &str, savedata: &SaveData) -> Result<(), String> {
    json::to_file(Path::new(path), savedata)
}

fn time_format(date: &DateTime<Local>) -> String {
    return date.format("%Y-%m-%d %H:%M:%S").to_string();
}

// Do maybe:
// TODO: Proper Error in Results instead of String
// TODO: Proper path representation instead of string
