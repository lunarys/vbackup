use crate::modules::object::*;
use crate::util::io::{file,json};
use crate::util::helper::{controller as controller_helper,check as check_helper};
use crate::modules;
use crate::modules::traits::{Controller, Sync, Check, Backup, Reporting};
use crate::modules::sync::SyncModule;
use crate::modules::controller::ControllerModule;
use crate::modules::backup::BackupModule;
use crate::modules::check::{Reference,CheckModule};
use crate::modules::reporting::ReportingModule;

use crate::{try_option};

use argparse::{ArgumentParser, Store, StoreOption, StoreTrue};
use std::path::Path;
use std::time::SystemTime;
use serde_json::Value;
use std::collections::HashMap;
use std::ops::Add;
use core::borrow::Borrow;
use chrono::{DateTime, Local};

pub fn main() -> Result<(),String> {
    let mut operation = String::new();
    let mut args = Arguments {
        dry_run: false,
        verbose: false,
        debug: false,
        quiet: false,
        force: false,
        name: None,
        base_config: String::from("/etc/vbackup/config.json"),
        no_docker: false
    };

    {
        let mut parser = ArgumentParser::new();
        parser.set_description("Client to interact with a MQTT device controller");
        parser.refer(&mut operation)
            .add_argument("operation", Store, "Operation to perform (run,backup,sync,list)")
            .required();
        parser.refer(&mut args.name)
            .add_option(&["-n", "--name"], StoreOption, "Name of the specific backup to run");
        parser.refer(&mut args.base_config)
            .add_option(&["-c", "--config"], Store, "Change base configuration file path");
        parser.refer(&mut args.dry_run)
            .add_option(&["--dry-run"], StoreTrue, "Print actions instead of performing them");
        parser.refer(&mut args.verbose)
            .add_option(&["-v", "--verbose", "--trace"], StoreTrue, "Print additional trace information");
        parser.refer(&mut args.debug)
            .add_option(&["-d", "--debug"], StoreTrue, "Print additional debug information");
        parser.refer(&mut args.quiet)
            .add_option(&["-q", "--quiet"], StoreTrue, "Only print warnings and errors");
        parser.refer(&mut args.force)
            .add_option(&["-f", "--force"], StoreTrue, "Force the run to disregard time constraints");
        parser.refer(&mut args.no_docker)
            .add_option(&["-b", "--bare", "--no-docker"], StoreTrue, "'Bare' run without using docker");
        parser.parse_args_or_exit();
    }

    let base_paths = json::from_file::<PathBase>(Path::new(args.base_config.as_str()))?;
    let paths = Paths::from(base_paths);

    let timeframes = json::from_file::<TimeFrames>(Path::new(&paths.timeframes_file))?;
    let mut reporter = match operation.as_str() {
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
    reporter.report(None, operation.as_str());

    let result = match operation.as_str() {
        "run" => backup_wrapper(&args, &paths, &timeframes, &reporter).and(sync_wrapper(&args, &paths, &timeframes, &reporter)),
        "backup" => backup_wrapper(&args, &paths, &timeframes, &reporter),
        "save" => sync_wrapper(&args, &paths, &timeframes, &reporter),
        "list" => list(&args, &paths),
        unknown => {
            let err = format!("Unknown operation: '{}'", unknown);
            //throw!(err);
            Err(err)
        }
    };

    reporter.report(None, "done");
    // TODO: Report size of backup directory?
    reporter.clear();
    return result;
}

fn backup_wrapper(args: &Arguments, paths: &Paths, timeframes: &TimeFrames, reporter: &ReportingModule) -> Result<(),String> {
    for mut config in get_config_list(args, paths)? {
        let module_paths = paths.for_module(config.name.as_str(), "backup", &config.original_path, &config.store_path);
        let mut savedata = get_savedata(module_paths.save_data.as_str())?; // TODO: Return this error?

        if config.backup.is_some() {
            reporter.report(Some(&["backup", config.name.as_str()]), "starting");

            let backup_config = config.backup.take().unwrap();
            let result = backup(args, module_paths, &config, backup_config, &mut savedata, timeframes);
            match result {
                Ok(true) => {
                    info!("Backup for '{}' was successfully executed", config.name.as_str());
                    reporter.report(Some(&["backup", config.name.as_str()]), "success");
                },
                Ok(false) => {
                    info!("Backup for '{}' was not executed due to constraints", config.name.as_str());
                    reporter.report(Some(&["backup", config.name.as_str()]), "skipped");
                },
                Err(err) => {
                    error!("Backup for '{}' failed: {}", config.name.as_str(), err);
                    reporter.report(Some(&["backup", config.name.as_str()]), "failed");
                }
            }
        } else {
            info!("No backup is configured for '{}'", config.name.as_str());
        }
    }

    // Only fails if some general configuration is not available, failure in single backups is only logged
    Ok(())
}

fn backup(args: &Arguments, paths: ModulePaths, config: &Configuration, backup_config: BackupConfiguration, savedata: &mut SaveData, timeframes: &TimeFrames) -> Result<bool,String> {
    let mut module: BackupModule = modules::backup::get_module(backup_config.backup_type.as_str())?;

    // Timing check
    let current_time = SystemTime::now();
    let mut queue_refs: Vec<&TimeFrameReference> = vec![];
    let mut queue_frame_entry: Vec<(&TimeFrame, Option<&TimeEntry>)> = vec![];

    // Init additional check
    let mut check_module = if !args.force {
        check_helper::init(&args, &paths.base_paths, &config, &backup_config.check, Reference::Backup)?
    } else {
        // No additional check is required if forced run
        None
    };

    if args.force {
        // Run is forced
        info!("Forcing run of '{}' backup", config.name.as_str());
    }

    // Fill queue with timeframes to run backup for
    for timeframe_ref in &backup_config.timeframes {
        // if amount of saves is zero, just skip it
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
        let last_backup = savedata.lastsync.get(&timeframe.identifier);

        // Only actually check if the run is not forced (continue loop for not saving)
        if !args.force {
            if last_backup.is_some() {
                if last_backup.unwrap().timestamp.add(timeframe.interval).lt(&current_time) {
                    // do sync
                    debug!("Backup for '{}' is required in timeframe '{}' considering the interval only", config.name.as_str(), timeframe_ref.frame.as_str());
                } else {
                    // don't sync
                    info!("Backup for '{}' is not executed in timeframe '{}' due to the interval", config.name.as_str(), timeframe_ref.frame.as_str());
                    continue;
                }
            } else {
                // Probably the first backup in this timeframe, just do it
                info!("This is probably the first backup run in timeframe '{}' for '{}', interval check is skipped", timeframe_ref.frame.as_str(), config.name.as_str());
            }

            // If this point of the loop is reached, only additional check is left to run
            if check_module.is_some() {
                if check_helper::run(&check_module, timeframe, &last_backup)? {
                    debug!("Backup for '{}' is required in timeframe '{}' considering the additional check", config.name.as_str(), timeframe_ref.frame.as_str());
                } else {
                    info!("Backup for '{}' is not executed in timeframe '{}' due to the additional check", config.name.as_str(), timeframe_ref.frame.as_str());
                    continue;
                }
            }
        } else {
            debug!("Run in timeframe '{}' is forced", timeframe_ref.frame.as_str())
        }

        // Do the backup for this one!
        queue_refs.push(timeframe_ref);
        queue_frame_entry.push((timeframe, last_backup));
    }

    // Is any backup required?
    if queue_refs.is_empty() {
        return Ok(false);
    } else if check_module.is_none() && !args.force {
        // Print this here to not have it over and over from the loop
        debug!("There is no additional check for the backup of '{}', only using the interval checks", config.name.as_str());
    }

    debug!("Executing backup for '{}'", config.name.as_str());

    // Save value from paths
    let save_data_path = paths.save_data.clone();

    // Do backup (all at once to enable optimizations)
    trace!("Invoking backup module");
    module.init(&config.name, &backup_config.config, paths, args.dry_run, args.no_docker)?;
    let backup_result = module.backup(&current_time, &queue_refs);
    trace!("Backup module is done");

    // Update check and savedata
    if backup_result.is_ok() {
        let time_this_save = current_time.clone();
        let date_this_save = DateTime::<Local>::from(time_this_save.clone());

        for (frame, entry_opt) in queue_frame_entry {
            // Update check state
            if !args.dry_run {
                trace!("Invoking state update for additional check in timeframe '{}'", frame.identifier.as_str());
                if let Err(err) = check_helper::update(&check_module, frame, &entry_opt) {
                    error!("State update for additional check in timeframe '{}' failed ({})", frame.identifier.as_str(), err);
                }
            } else {
                // TODO: dry-run
            }

            let time_next_save = current_time.clone().add(frame.interval);
            let date_next_save = DateTime::<Local>::from(time_next_save.clone());

            // Update savedata
            savedata.lastsave.insert(frame.identifier.clone(), TimeEntry {
                timestamp: time_this_save.clone(),
                date: Some(time_format(&date_this_save))
            });

            savedata.nextsave.insert(frame.identifier.clone(), TimeEntry {
                timestamp: time_next_save.clone(),
                date: Some(time_format(&date_next_save))
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
            // TODO: dry-run
        }
    }

    // Check can be freed as it is not required anymore
    if let Err(err) = check_helper::clear(&mut check_module) {
        error!("Could not clear the check module: {}", err);
    }

    // Free backup module
    if let Err(err) = module.clear() {
        error!("Could not clear backup module: {}", err);
    }

    return backup_result.map(|_| true);
}

fn sync_wrapper(args: &Arguments, paths: &Paths, timeframes: &TimeFrames, reporter: &ReportingModule) -> Result<(),String> {
    for mut config in get_config_list(args, paths)? {
        let module_paths = paths.for_module(config.name.as_str(), "sync", &config.original_path, &config.store_path);
        let mut savedata = get_savedata(module_paths.save_data.as_str())?; // TODO: Return this error?

        if config.sync.is_some() {
            reporter.report(Some(&["sync", config.name.as_str()]), "starting");

            let sync_config = config.sync.take().unwrap();
            let result = sync(args, module_paths, &config, sync_config, &mut savedata, timeframes);
            match result {
                Ok(true) => {
                    info!("Sync for '{}' was successfully executed", config.name.as_str());
                    reporter.report(Some(&["sync", config.name.as_str()]), "success");
                },
                Ok(false) => {
                    info!("Sync for '{}' was not executed due to constraints", config.name.as_str());
                    reporter.report(Some(&["sync", config.name.as_str()]), "skipped");
                },
                Err(err) => {
                    error!("Sync for '{}' failed: {}", config.name.as_str(), err);
                    reporter.report(Some(&["sync", config.name.as_str()]), "failed");
                }
            }
        } else {
            info!("No sync is configured for '{}'", config.name.as_str());
        }
    }

    // Only fails if some general configuration is not available, failure in single syncs is only logged
    Ok(())
}

fn sync(args: &Arguments, paths: ModulePaths, config: &Configuration, sync_config: SyncConfiguration, savedata: &mut SaveData, timeframes: &TimeFrames) -> Result<bool,String> {
    let mut module: SyncModule = modules::sync::get_module(sync_config.sync_type.as_str())?;

    // Timing check
    let current_time = SystemTime::now();
    let last_sync = savedata.lastsync.get(&sync_config.interval.frame);
    let timeframe: &TimeFrame = try_option!(timeframes.get(&sync_config.interval.frame), "Referenced timeframe for sync does not exist");

    let base_paths = paths.base_paths.borrow();
    let mut check_module = if !args.force {
        check_helper::init(&args, base_paths, &config, &sync_config.check, Reference::Sync)?
    } else {
        // No additional check is required if forced run
        None
    };

    if !args.force {
        if last_sync.is_some() {
            if last_sync.unwrap().timestamp.add(timeframe.interval).lt(&current_time) {
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
            if check_helper::run(&check_module, timeframe, &last_sync)? {
                debug!("Sync for '{}' is required considering the additional check", config.name.as_str());
            } else {
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

    debug!("Executing sync for '{}'", config.name.as_str());

    // Initialize sync module
    let save_data_path = paths.save_data.clone();
    module.init(&config.name, &sync_config.config, paths, args.dry_run, args.no_docker)?;

    // Set up controller (if configured)
    let mut controller_module = controller_helper::init(&args, base_paths, &config, &sync_config.controller)?;
    if !args.dry_run {
        trace!("Invoking remote device controller");
        if controller_helper::start(&controller_module)? {
            // There is no controller or device is ready for sync
            info!("");
        } else {
            // Device did not start before timeout or is not available
            warn!("Remote device is not available, aborting sync");
            return Ok(false);
        }
    } else {
        // TODO: dry-run
    }

    // Run backup
    let sync_result = if !args.dry_run {
        trace!("Invoking sync module");
        let sync_result = module.sync();
        trace!("Sync module is done");
        if sync_result.is_ok() {
            // Update internal state of check
            trace!("Invoking state update for additional check in timeframe '{}'", timeframe.identifier.as_str());
            if let Err(err) = check_helper::update(&check_module, timeframe, &last_sync) {
                error!("State update for additional check in timeframe '{}' failed ({})", timeframe.identifier.as_str(), err);
            }

            // Update save data
            let date = DateTime::<Local>::from(current_time.clone());
            savedata.lastsync.insert(timeframe.identifier.clone(), TimeEntry {
                timestamp: current_time,
                date: Some(time_format(&date))
            });
        } else {
            error!("Sync failed, cleaning up");
        }
        sync_result
    } else {
        // TODO: dry-run
        Ok(())
    };

    // Run controller end (result is irrelevant here)
    if !args.dry_run {
        if let Err(err) = controller_helper::end(&controller_module) {
            error!("Stopping the remote device after use failed");
        }
    } else {
        // TODO: dry-run
    }

    // Write savedata update only if sync was successful
    if sync_result.is_ok() {
        if !args.dry_run {
            trace!("Writing new savedata to '{}'", save_data_path.as_str());
            if let Err(err) = write_savedata(save_data_path.as_str(), savedata) {
                error!("Could not update savedata for '{}' sync ({})", config.name.as_str(), err);
            }
        } else {
            // TODO: dry-run
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

pub fn list(args: &Arguments, paths: &Paths) -> Result<(),String> {
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

    fn print_timeframe_ref(frame: &TimeFrameReference, with_amount: bool) {
        print!("       - {}", frame.frame);
        if with_amount {
            print!(", maximal amount: {}", frame.amount);
        }
        println!();
    }

    println!("vBackup configurations:");

    for mut config in get_config_list(args, paths)? {
        let backup_paths = paths.for_module(config.name.as_str(), "backup", &config.original_path, &config.store_path);
        let sync_paths = paths.for_module(config.name.as_str(), "sync", &config.original_path, &config.store_path);

        let backup_path = backup_paths.original_path.unwrap_or(String::from("Not configured"));

        println!("- Configuration for: {} is {}", config.name.as_str(), if config.disabled {"disabled"} else {"enabled"});

        if let Some(backup_config) = config.backup.as_ref() {
            println!("   + Backup of type '{}' is {}", backup_config.backup_type, if backup_config.disabled {"disabled"} else {"enabled"});

            if !backup_config.disabled {
                println!("     * Original data path: {}", backup_path);
                println!("     * Backup data path:   {}", sync_paths.store_path);

                println!("     * Timeframes for backup:");
                backup_config.timeframes.iter().for_each(|f| print_timeframe_ref(f, true));

                print_check(&backup_config.check)
            }
        } else {
            println!("   x No backup configured");
        }

        if let Some(sync_config) = config.sync.as_ref() {
            println!("   + Sync of type '{}' is {}", sync_config.sync_type, if sync_config.disabled {"disabled"} else {"enabled"});

            if !sync_config.disabled {
                println!("     * Path of synced data: {}", sync_paths.store_path);

                println!("     * Interval for sync:");
                print_timeframe_ref(&sync_config.interval, false);

                print_check(&sync_config.check);
                print_controller(&sync_config.controller);
            }
        } else {
            println!("   x No sync configured");
        }
    }

    Ok(())
}

fn get_config_list(args: &Arguments, paths: &Paths) -> Result<Vec<Configuration>, String> {
    let volume_config_path = format!("{}/volumes", &paths.config_dir);

    let files = if args.name.is_some() {
        // Only run this one
        let path = format!("{}/{}.json", volume_config_path, args.name.as_ref().unwrap());
        vec![Path::new(&path).to_path_buf()]
    } else {
        // Run all
        file::list_in_dir(volume_config_path.as_str())?
    };

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
    let savedata = if file::exists(path) {
        json::from_file::<SaveData>(Path::new(path))?
    } else {
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

// TODO: Proper dry-run implementation
// TODO: Prepare docker image
// TODO: Lock for single instance

// Do maybe:
// TODO: Proper Error in Results instead of String
// TODO: Proper path representation instead of string
