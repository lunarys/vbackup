use crate::util::io::{file,json};
use crate::modules::reporting::{ReportingModule, ReportingWrapper};
use crate::util::objects::time::{TimeFrameReference};
use crate::util::objects::paths::{Paths,PathBase,ModulePaths,SourcePath};
use crate::util::objects::configuration::Configuration;
use crate::util::objects::reporting::{OperationStatus, ReportEvent};
use crate::processing::{preprocessor,scheduler,processor};
use crate::{Arguments, restore};

use crate::{log_error};

use std::path::Path;
use serde_json::Value;
use std::rc::Rc;

pub fn main(mut args: Arguments) -> Result<(),String> {
    let base_paths = json::from_file::<PathBase>(Path::new(args.base_config.as_str()))?;
    args.no_docker = args.no_docker || base_paths.no_docker;
    let paths = Rc::new(Paths::from(base_paths));

    if args.no_docker {
        warn!("Running without docker, this only supports limited features");
    }

    file::create_dir_if_missing(paths.save_dir.as_str(), true)?;
    file::create_dir_if_missing(paths.tmp_dir.as_str(), true)?;

    // List does not need anything else
    if args.operation == "list" {
        return list(&Rc::new(args), &paths);
    }

    // Set up reporter (if existing)
    let mut reporter = if args.no_reporting {
        ReportingModule::new_empty()
    } else if let Some(reporter_config) = json::from_file_checked::<Value>(Path::new(paths.reporting_file.as_str()))? {
        let result = ReportingModule::new_combined(&reporter_config, &paths, &args);
        match result {
            Ok(mut module) => {
                match module.init() {
                    Ok(_) => module,
                    Err(err) => {
                        return Err(format!("Could not initialize combined reporter: {}", err));
                    }
                }
            },
            Err(err) => {
                return Err(format!("Could not create combined reporter: {}", err));
            }
        }
    } else {
        ReportingModule::new_empty()
    };

    // Only actually does something if run, backup or sync
    // TODO: Might solve this via RunType also
    reporter.report_operation(OperationStatus::START(args.operation.clone()));
    reporter.report_raw(ReportEvent::Version(String::from(env!("CARGO_PKG_VERSION"))));

    let result = if args.operation == "restore" {
        args.is_restore = true;
        restore::main(Rc::new(args), paths)
    } else {
        let (do_backup, do_sync) = match args.operation.as_str() {
            "run" => Ok((true, true)),
            "backup" | "save" => Ok((true, false)),
            "sync" => Ok((false, true)),
            unknown => {
                Err(format!("Unknown operation: '{}'", unknown))
            }
        }?;

        let args = Rc::new(args);

        let config_list = get_config_list(&args, paths.as_ref())?;
        let preprocessed = preprocessor::preprocess(config_list, &args, &paths, &mut reporter, do_backup, do_sync)?;
        let scheduled = scheduler::get_exec_order(preprocessed.configurations)?;
        let result = processor::process_configurations(&args, &mut reporter, scheduled, preprocessed.savedata);
        result
    };

    reporter.report_operation(OperationStatus::DONE);
    log_error!(reporter.clear());
    return result;
}

pub fn get_config_list(args: &Rc<Arguments>, paths: &Paths) -> Result<Vec<Configuration>, String> {
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

pub fn list(args: &Rc<Arguments>, paths: &Rc<Paths>) -> Result<(), String> {

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
    println!("vbackup configurations:");

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
                match backup_paths.source {
                    SourcePath::Single(path) => {
                        println!("     * Original data path: {}", path.as_str());
                    },
                    SourcePath::Multiple(paths) => {
                        println!("     * Original data paths:");
                        for path in paths {
                            println!("       {}: {}", path.name, path.path);
                        }
                    }
                }

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
                match sync_paths.source {
                    SourcePath::Single(path) => {
                        println!("     * Path of synced data: {}", path.as_str());
                    },
                    SourcePath::Multiple(paths) => {
                        println!("     * Path of synced data:");
                        for path in paths {
                            println!("       {}: {}", path.name, path.path);
                        }
                    }
                }

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
