use crate::util::io::{file,json};
use crate::modules;
use crate::modules::traits::{Reporting};
use crate::modules::sync::SyncModule;
use crate::modules::backup::BackupModule;
use crate::modules::check::Reference;
use crate::modules::reporting::ReportingModule;
use crate::util::io::savefile::{get_savedata};
use crate::util::objects::time::{TimeFrames, TimeFrameReference};
use crate::util::objects::paths::{Paths,PathBase,ModulePaths};
use crate::util::objects::configuration::Configuration;
use crate::processing::{preprocessor,scheduler,processor};
use crate::processing::{backup,sync};
use crate::Arguments;

use crate::{try_option, dry_run,log_error};

use std::path::Path;
use serde_json::Value;
use std::collections::HashMap;
use std::ops::Add;
use core::borrow::Borrow;
use chrono::{DateTime, Local, Duration};
use std::rc::Rc;

pub fn main(args: Arguments) -> Result<(),String> {
    let base_paths = json::from_file::<PathBase>(Path::new(args.base_config.as_str()))?;
    let paths = Rc::new(Paths::from(base_paths));

    file::create_dir_if_missing(paths.save_dir.as_str(), true)?;
    file::create_dir_if_missing(paths.tmp_dir.as_str(), true)?;

    // List does not need anything else
    if args.operation == "list" {
        return list(&args, &paths);
    }

    // Load timeframes from file
    let timeframes = json::from_file::<TimeFrames>(Path::new(&paths.timeframes_file))?;

    // Set up reporter (if existing)
    let mut reporter = if let Some(reporter_config) = json::from_file_checked::<Value>(Path::new(paths.reporting_file.as_str()))? {
        let mut r = ReportingModule::new_combined();
        r.init(&reporter_config, &paths, &args)?;
        r
    } else {
        ReportingModule::new_empty()
    };

    // Only actually does something if run, backup or sync
    log_error!(reporter.report(None, args.operation.as_str()));

    let config_list = get_config_list(&args, paths.as_ref())?;
    let preprocessed = preprocessor::preprocess(config_list, &args, &paths)?;
    let scheduled = scheduler::get_exec_order(preprocessed)?;
    let result = processor::process_configurations(&args, &reporter, scheduled);

    /*
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
        "backup" | "save" => {
            let result = backup_wrapper(&args, &paths, &timeframes, &reporter);
            if let Ok((original_size, backup_size)) = result.as_ref() {
                log_error!(reporter.report(Some(&["size", "original"]), original_size.to_string().as_str()));
                log_error!(reporter.report(Some(&["size", "backup"]), backup_size.to_string().as_str()));
            }
            result.map(|_| ())
        },
        "sync" => {
            let result = sync_wrapper(&args, &paths, &timeframes, &reporter);
            if let Ok(sync_size) = result.as_ref() {
                log_error!(reporter.report(Some(&["size", "sync"]), sync_size.to_string().as_str()));
            }
            result.map(|_| ())
        },
        unknown => {
            let err = format!("Unknown operation: '{}'", unknown);
            //throw!(err);
            Err(err)
        }
    };
    */

    log_error!(reporter.report(None, "done"));
    log_error!(reporter.clear());
    return result;
}

pub fn get_config_list(args: &Arguments, paths: &Paths) -> Result<Vec<Configuration>, String> {
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

fn backup_wrapper(args: &Arguments, paths: &Rc<Paths>, timeframes: &TimeFrames, reporter: &ReportingModule) -> Result<(u64,u64),String> {
    // Collect total sizes of the backup
    let mut original_size_acc = 0;
    let mut backup_size_acc = 0;

    // Go through all configurations in the config directory
    for mut config in get_config_list(args, paths)? {

        // Check if the while configuration is disabled
        if config.disabled {
            info!("Configuration for '{}' is disabled, skipping backup", config.name.as_str());
            let report_result = reporter.report(Some(&["backup", config.name.as_str()]), "disabled");
            log_error!(report_result);
            continue;
        }

        // Get paths specifically for this module
        let module_paths = ModulePaths::for_backup_module(paths, "backup", &config);

        // Only do something else if a backup is present in this configuration
        if config.backup.is_some() {

            // Take ownership of config
            let backup_config = config.backup.take().unwrap();

            // Check if this backup is disabled
            if backup_config.disabled {
                info!("Backup for '{}' is disabled", config.name.as_str());
                let report_result = reporter.report(Some(&["backup", config.name.as_str()]), "disabled");
                log_error!(report_result);
                continue;
            }

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

            // Save those paths for later, as the ModulePaths will be moved
            let original_path = module_paths.source.clone();
            let store_path = module_paths.destination.clone();

            // Run the backup and evaluate the result
            // let result = backup::backup(args, module_paths, &config, backup_config, &mut savedata, timeframes);
            let result = Err(String::from("OLD VERSION"));
            match result {
                Ok(true) => {
                    info!("Backup for '{}' was successfully executed", config.name.as_str());
                    let report_result = reporter.report(Some(&["backup", config.name.as_str()]), "success");
                    log_error!(report_result);
                },
                Ok(false) => {
                    info!("Backup for '{}' was not executed due to constraints", config.name.as_str());
                    let report_result = reporter.report(Some(&["backup", config.name.as_str()]), "skipped");
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
        } else {
            info!("No backup is configured for '{}'", config.name.as_str());
        }
    }

    // Only fails if some general configuration is not available, failure in single backups is only logged
    Ok((original_size_acc, backup_size_acc))
}



fn sync_wrapper(args: &Arguments, paths: &Rc<Paths>, timeframes: &TimeFrames, reporter: &ReportingModule) -> Result<u64,String> {
    // Collect the total size of synchronized files
    let mut acc_size = 0;

    // Go through all configurations in the config directory
    for mut config in get_config_list(args, paths)? {

        // Check if the while configuration is disabled
        if config.disabled {
            info!("Configuration for '{}' is disabled, skipping sync", config.name.as_str());
            let report_result = reporter.report(Some(&["sync", config.name.as_str()]), "disabled");
            log_error!(report_result);
            continue;
        }

        // Get paths specifically for this module
        let module_paths = ModulePaths::for_sync_module(paths, "sync", &config);

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

            // Save owned objects of configuration and path
            let sync_config = config.sync.take().unwrap();
            let store_path = module_paths.source.clone();

            // Check if the while configuration is disabled
            if sync_config.disabled {
                info!("Sync for '{}' is disabled", config.name.as_str());
                let report_result = reporter.report(Some(&["sync", config.name.as_str()]), "disabled");
                log_error!(report_result);
                continue;
            }

            // Announce that this sync is starting
            log_error!(reporter.report(Some(&["sync", config.name.as_str()]), "starting"));

            // Run the backup and evaluate the result
            //let result = sync::sync(args, module_paths, &config, sync_config, &mut savedata, timeframes);
            let result = Err(String::from("OLD VERSION"));
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
        } else {
            info!("No sync is configured for '{}'", config.name.as_str());
        }
    }

    // Only fails if some general configuration is not available, failure in single syncs is only logged
    Ok(acc_size)
}

pub fn list(args: &Arguments, paths: &Rc<Paths>) -> Result<(), String> {

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
        let backup_paths = ModulePaths::for_backup_module(paths, "backup", &config);
        let sync_paths = ModulePaths::for_sync_module(paths, "sync", &config);

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

// Do maybe:
// TODO: Proper Error in Results instead of String
// TODO: Proper path representation instead of string
