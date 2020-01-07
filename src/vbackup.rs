use crate::modules::object::*;
use crate::util::io::json;
use crate::modules;

use crate::{try_option};

use argparse::{ArgumentParser, Store, StoreOption, StoreTrue};
use std::path::Path;
use crate::modules::traits::{Controller, Sync, Check, Backup};
use crate::modules::sync::SyncModule;
use crate::modules::controller::ControllerModule;
use crate::modules::backup::BackupModule;
use crate::modules::check::CheckModule;
use crate::util::io::file;
use crate::util::helper::{controller as controller_helper,check as check_helper};
use std::time::SystemTime;
use serde_json::Value;
use std::collections::HashMap;
use std::ops::Add;
use core::borrow::Borrow;

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

    let result = match operation.as_str() {
        "run" => backup_wrapper(&args, &paths, &timeframes).and(sync_wrapper(&args, &paths, &timeframes)),
        "backup" => backup_wrapper(&args, &paths, &timeframes),
        "save" => sync_wrapper(&args, &paths, &timeframes),
        "list" => list(&args, &paths),
        unknown => {
            let err = format!("Unknown operation: '{}'", unknown);
            //throw!(err);
            Err(err)
        }
    };

    return result;
}

fn backup_wrapper(args: &Arguments, paths: &Paths, timeframes: &TimeFrames) -> Result<(),String> {
    for mut config in get_config_list(args, paths)? {
        let module_paths = paths.for_module(config.name.as_str(), "backup", &config.original_path, &config.store_path);
        let savedata = get_savedata(module_paths.save_data.as_str())?; // TODO: Return this error?

        if config.backup.is_some() {
            let backup_config = config.backup.take().unwrap();
            let result = backup(args, module_paths, &config, backup_config, &savedata, timeframes);
            match result {
                Ok(true) => info!(""),
                Ok(false) => info!(""),
                Err(err) => error!("")
            }
        } else {
            info!("");
        }
    }

    // Only fails if some general configuration is not available, failure in single backups is only logged
    Ok(())
}

fn backup(args: &Arguments, paths: ModulePaths, config: &Configuration, backup_config: BackupConfiguration, savedata: &SaveData, timeframes: &TimeFrames) -> Result<bool,String> {
    let mut module: BackupModule = modules::backup::get_module(backup_config.backup_type.as_str())?;

    // Timing check
    let current_time = SystemTime::now();
    let mut queue_refs: Vec<&TimeFrameReference> = vec![];
    let mut queue_frame_entry: Vec<(&TimeFrame, Option<&TimeEntry>)> = vec![];

    // Init additional check
    let mut check_module = if !args.force {
        check_helper::init(&args, &paths.base_paths, &config, &backup_config.check)?
    } else {
        // No additional check is required if forced run
        None
    };

    if args.force {
        // Run is forced
        info!("");
    }

    // Fill queue with timeframes to run backup for
    for timeframe_ref in &backup_config.timeframes {
        let timeframe_opt = timeframes.get(&timeframe_ref.frame);
        let timeframe = if timeframe_opt.is_some() {
            timeframe_opt.unwrap()
        } else {
            error!("Referenced timeframe '{}' for backup does not exist", &timeframe_ref.frame);
            continue;
        };

        let last_backup = savedata.lastsync.get(&timeframe.identifier);

        // Only actually check if the run is not forced (continue loop for not saving)
        if !args.force {
            if last_backup.is_some() {
                if last_backup.unwrap().timestamp.add(timeframe.interval).lt(&current_time) {
                    // do sync
                    debug!("");
                } else {
                    // don't sync
                    info!("");
                    continue;
                }
            } else {
                // Probably the first backup in this timeframe, just do it
                info!("");
            }

            // If this point of the loop is reached, only additional check is left to run
            if check_helper::run(&check_module, timeframe, &last_backup)? {
                debug!("");
            } else {
                info!("");
                continue;
            }
        }

        // Do the backup for this one!
        queue_refs.push(timeframe_ref);
        queue_frame_entry.push((timeframe, last_backup));
    }

    // Is any backup required?
    if queue_refs.is_empty() {
        return Ok(false);
    }

    // Save value from paths
    let save_data_path = paths.save_data.clone();

    // Do backup (all at once to enable optimizations)
    module.init(&config.name, &backup_config.config, paths, args.dry_run, args.no_docker)?;
    let backup_result = module.backup(&queue_refs);

    // Update check state
    for (frame, entry_opt) in queue_frame_entry {
        if check_helper::update(&check_module, frame, &entry_opt).is_err() {
            error!("");
        }
    }

    // Check can be freed as it is not required anymore
    if check_helper::clear(&mut check_module).is_err() {
        error!("");
    }

    // Free backup module
    if module.clear().is_err() {
        error!("");
    }

    // Write savedata update
    if write_savedata(save_data_path.as_str(), savedata).is_err() {
        error!("");
    }

    return backup_result.map(|_| true);
}

fn sync_wrapper(args: &Arguments, paths: &Paths, timeframes: &TimeFrames) -> Result<(),String> {
    for mut config in get_config_list(args, paths)? {
        let module_paths = paths.for_module(config.name.as_str(), "sync", &config.original_path, &config.store_path);
        let savedata = get_savedata(module_paths.save_data.as_str())?; // TODO: Return this error?

        if config.sync.is_some() {
            let sync_config = config.sync.take().unwrap();
            let result = sync(args, module_paths, &config, sync_config, &savedata, timeframes);
            match result {
                Ok(true) => info!(""),
                Ok(false) => info!(""),
                Err(err) => error!("")
            }
        } else {
            info!("")
        }
    }

    // Only fails if some general configuration is not available, failure in single syncs is only logged
    Ok(())
}

fn sync(args: &Arguments, paths: ModulePaths, config: &Configuration, sync_config: SyncConfiguration, savedata: &SaveData, timeframes: &TimeFrames) -> Result<bool,String> {
    let mut module: SyncModule = modules::sync::get_module(sync_config.sync_type.as_str())?;

    // Timing check
    let current_time = SystemTime::now();
    let last_sync = savedata.lastsync.get(&sync_config.interval.frame);
    let timeframe: &TimeFrame = try_option!(timeframes.get(&sync_config.interval.frame), "Referenced timeframe for sync does not exist");

    if !args.force {
        if last_sync.is_some() {
            if last_sync.unwrap().timestamp.add(timeframe.interval).lt(&current_time) {
                // do sync
                debug!("");
            } else {
                // sync not necessary
                info!("");
                return Ok(false);
            }
        } else {
            // This is probably the first sync, so just do it
            info!("");
        }
    } else {
        // Run is forced
        info!("")
    }

    // Initialize sync module
    let save_data_path = paths.save_data.clone();
    let base_paths = paths.base_paths.borrow();
    module.init(&config.name, &sync_config.config, paths, args.dry_run, args.no_docker)?;

    // Set up additional check and controller (if configured)
    let mut controller_module = controller_helper::init(&args, base_paths, &config, &sync_config.controller)?;
    let mut check_module = if !args.force {
        check_helper::init(&args, base_paths, &config, &sync_config.check)?
    } else {
        // No additional check is required if forced run
        None
    };

    // Run additional check
    if !check_helper::run(&check_module, timeframe, &last_sync)? {
        debug!("");
        return Ok(false);
    }

    // Run backup
    let sync_result = module.sync();
    debug!("");
    if sync_result.is_ok() {
        // Update internal state of check
        if check_helper::update(&check_module, timeframe, &last_sync).is_err() {
            error!("");
        }
    } else {
        error!("");
    }

    // Check can be freed as it is not required anymore
    if check_helper::clear(&mut check_module).is_err() {
        error!("");
    }

    // Run controller end (result is irrelevant here)
    if controller_helper::end(&controller_module).is_err() {
        error!("");
    }

    // Controller can be freed as it is not required anymore
    if controller_helper::clear(&mut controller_module).is_err() {
        error!("");
    }

    // Free sync module
    if module.clear().is_err() {
        error!("");
    }

    // Write savedata update
    if write_savedata(save_data_path.as_str(), savedata).is_err() {
        error!("");
    }

    // Return Ok(true) for sync was executed or Err(error) for failed sync
    return sync_result.map(|_| true);
}

pub fn list(args: &Arguments, paths: &Paths) -> Result<(),String> {
    unimplemented!()
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

// TODO: Reporting
// TODO: Error logging when thrown (with reporting?)
// TODO: Proper Error in Results instead of String
// TODO: Proper path representation instead of string
// TODO: Proper dry-run implementation
