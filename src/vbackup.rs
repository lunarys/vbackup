use crate::modules::object::*;
use crate::util::io::json;
use crate::modules;

use crate::{try_option, change_result};

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

    let timeframes = json::from_file::<Timeframes>(Path::new(&paths.timeframes_file))?;

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

fn backup_wrapper(args: &Arguments, paths: &Paths, timeframes: &Timeframes) -> Result<(),String> {
    for mut config in get_config_list(args, paths)? {
        let module_paths = paths.for_module(config.name.as_str(), "backup", &config.original_path, &config.store_path);
        let savedata = get_savedata(&module_paths)?;

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

fn backup(args: &Arguments, paths: ModulePaths, config: &Configuration, backup_config: BackupConfiguration, savedata: &SaveData, timeframes: &Timeframes) -> Result<bool,String> {
    // TODO: Timings check
    let module: BackupModule = modules::backup::get_module(backup_config.backup_type.as_str())?;

    // Timing check
    let current_time = SystemTime::now();
    // TODO: list of timeframes to save in
    if !args.force {
        for timeframe_ref in &backup_config.timeframes {
            let timeframe_opt = timeframes.get(&timeframe_ref.frame);
            let timeframe = if timeframe_opt.is_some() {
                timeframe_opt.unwrap()
            } else {
                error!("Referenced timeframe for backup does not exist");
                continue;
            };

            // TODO
            //let last_backup_time = savedata.;
            //if last_backup_time +
        }
    } else {
        // Run is forced
        info!("")
    }

    // TODO

    Ok(true)
}

fn sync_wrapper(args: &Arguments, paths: &Paths, timeframes: &Timeframes) -> Result<(),String> {
    for mut config in get_config_list(args, paths)? {
        let module_paths = paths.for_module(config.name.as_str(), "sync", &config.original_path, &config.store_path);
        let savedata = get_savedata(&module_paths)?;

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

fn sync(args: &Arguments, paths: ModulePaths, config: &Configuration, sync_config: SyncConfiguration, savedata: &SaveData, timeframes: &Timeframes) -> Result<bool,String> {
    let mut module: SyncModule = modules::sync::get_module(sync_config.sync_type.as_str())?;

    // Timing check
    let current_time = SystemTime::now();
    let last_sync = if savedata.lastsync.contains_key(&sync_config.interval.frame) {
        Some(savedata.lastsync.get(&sync_config.interval.frame).unwrap())
    } else {
        None
    };

    if !args.force {
        let timeframe: &Timeframe = try_option!(timeframes.get(&sync_config.interval.frame), "Referenced timeframe for sync does not exist");
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
    let base_paths = paths.base_paths.borrow();
    module.init(&config.name, &sync_config.config, paths, args.dry_run, args.no_docker)?;

    // Set up additional check and controller (if configured)
    let mut controller_module = controller_helper::init(&args, base_paths, &config, &sync_config.controller)?;
    let mut check_module = if !args.force {
        check_helper::init(&args, base_paths, &config, &sync_config.check, &last_sync)?
    } else {
        // No additional check is required if forced run
        None
    };

    // Run additional check
    if !check_helper::run(&check_module)? {
        debug!("");
        return Ok(false);
    }

    // Run backup
    let sync_result = module.sync();
    debug!("");
    if sync_result.is_ok() {
        // Update internal state of check
        if check_helper::update(&check_module).is_err() {
            error!("")
        }
    } else {
        error!("");
    }

    // Check can be freed as it is not required anymore
    if check_helper::clear(&mut check_module).is_err() {
        error!("")
    }

    // Run controller end (result is irrelevant here)
    if controller_helper::end(&controller_module).is_err() {
        error!("")
    }

    // Controller can be freed as it is not required anymore
    if controller_helper::clear(&mut controller_module).is_err() {
        error!("")
    }

    // TODO: Write savedata update

    // Return Ok(true) for sync was executed or Err(error) for failed sync
    change_result!(sync_result, true)
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

fn get_savedata(module_paths: &ModulePaths) -> Result<SaveData, String> {
    let savedata = if file::exists(module_paths.save_data.as_str()) {
        json::from_file::<SaveData>(Path::new(module_paths.save_data.as_str()))?
    } else {
        SaveData {
            lastsave: HashMap::new(),
            nextsave: HashMap::new(),
            lastsync: HashMap::new()
        }
    };

    return Ok(savedata);
}

// TODO: Proper Error in Results instead of String
// TODO: Proper path representation instead of string
// TODO: Proper dry-run implementation
