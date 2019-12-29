use crate::modules::object::*;
use crate::util::json;
use crate::modules;
use crate::conf_resolve;

use crate::{throw};

use argparse::{ArgumentParser, Store, StoreOption, StoreTrue};
use std::path::Path;
use crate::modules::traits::{Controller, Sync};
use crate::modules::sync::SyncModule;
use crate::modules::controller::ControllerModule;
use serde_json::Value;

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

    let base_paths = json::from_file::<PathBase>(args.base_config.as_str())?;
    let paths = Paths::from(base_paths);

    let result = match operation.as_str() {
        "run" => backup_wrapper(&args, &paths).and(sync_wrapper(&args, &paths)),
        "backup" => backup_wrapper(&args, &paths)
        "save" => sync_wrapper(&args, &paths),
        "list" => list(&args, &paths),
        unknown => throw!(format!("Unknown operation: '{}'", unknown))
    }

    return result;
}

fn backup_wrapper(args: &Arguments, paths: &Paths) -> Result<(),String> {
    let volume_config_path = format!("{}/volumes", &paths.config_dir);
    if args.name.is_some() {
        // Only run this one
    } else {
        // Run all
    }

    Ok(())
}

fn sync_wrapper(args: &Arguments, paths: &Paths) -> Result<(),String> {
    if config.syncs.is_some() {
        for sync in config.syncs.unwrap() {

        }
    } else {
        debug!("No sync configured for '{}'", &config.name)
    }

    let config_dir = Path::new(&paths.config_dir);
    let mut volume_config = config_dir.to_path_buf();
    volume_config.push("volumes");
    if args.name.is_some() {
        // Only run this one
        volume_config.set_file_name(args.name.as_ref().unwrap()); // TODO: First push required?
        volume_config.set_extension("json");
        if volume_config.exists() {
            let config = json::from_file::<Configuration>(volume_config.as_os_str())?;
            if config.disabled {
                info!(format!("Configuration for '{}' is disabled", config.name))
            } else {
                let only_sync = config.backup.is_none();
                let paths_for_module = paths.for_module(config.name.as_str(), "sync", &config.original_path, &config.store_path;
                sync(args, paths_for_module, )
            }
        } else {
            throw!(format!("Named volume config does not exist ({})", volume_config))
        }
    } else {
        // Run all
    }

    Ok(())
}

fn backup(args: &Arguments, paths: &Paths, config: &Configuration, backup_config: &BackupConfiguration) -> Result<(),String> {
    // TODO: Timings check

    let module_paths = paths.for_module(config.name.as_str(), "backup", &config.original_path, &config.store_path);
    let module = modules::backup::get_module(backup_config.backup_type.as_str())?;

    // TODO

    Ok(())
}

fn sync(args: &Arguments, paths: &Paths, config: &Configuration, sync_config: &SyncConfiguration) -> Result<bool,String> {
    // TODO: Base timing check

    let module_paths = paths.for_module(config.name.as_str(), "backup", &config.original_path, &config.store_path);
    let mut module = modules::sync::get_module(sync_config.sync_type.as_str())?;

    // Initialize sync module
    module.init(&config.name, &sync_config.config, module_paths)?;

    // Set up additional check and controller (if configured)
    let mut check_module = check_init(&args, &paths, &config, &sync_config.check)?;
    let mut controller_module = controller_init(&args, &paths, &config, &sync_config.controller)?;

    // Run additional check
    if !check_run(&check_module)? {
        debug!();
        return Ok(false);
    }

    // Run backup
    let sync_result = module.sync();
    debug!();
    if sync_result.is_ok() {
        // Update internal state of check
        if check_update(&check_module).is_err() {
            error!()
        }
    } else {
        error!();
    }

    // Check can be freed as it is not required anymore
    if check_clear(&mut check_module).is_err() {
        error!()
    }

    // Run controller end (result is irrelevant here)
    if controller_end(&controller_module).is_err() {
        error!()
    }

    // Controller can be freed as it is not required anymore
    if controller_clear(&mut controller_module).is_err() {
        error!()
    }

    // Return Ok(true) for sync was executed or Err(error) for failed sync
    change_result!(sync_result, true)
}

fn controller_init(args: &Arguments, paths: &Paths, config: &Configuration, controller_config: &Option<Value>) -> Result<Option<ControllerModule>,String> {
    if controller_config.is_some() {
        let controller_type = try_option!(controller_config.unwrap().get("type"), "Controller config contains no field 'type'");
        let module_paths = paths.for_module(config.name.as_str(), "controller", &config.original_path, &config.store_path);

        let mut module = modules::controller::get_module(controller_type.as_str())?;
        module.init(config.name.as_str(), &controller_config.unwrap(), module_paths, args.dry_run, args.no_docker)?;

        return Ok(Some(module));
    } else {
        return Ok(None);
    }
}

fn controller_start(module: &Option<ControllerModule>) -> Result<bool,String> {
    if module.is_some() {
        let result = module.unwrap().begin()?;
        if result {
            debug!();
        } else {
            debug!();
        }
        return Ok(result);
    }

    // No controller means sync can be started
    return Ok(true);
}

fn controller_end(module: &Option<ControllerModule>) -> Result<bool,String> {
    if module.is_some() {
        let result = module.unwrap().end()?;
        if result {
            debug!();
        } else {
            debug!();
        }
        return Ok(result);
    }

    return Ok(true);
}

fn controller_clear(module: &mut Option<ControllerModule>) -> Result<(),String> {
    if module.is_some() {
        module.unwrap().clear()?;
    }

    return Ok(());
}

fn check_init(args: &Arguments, paths: &Paths, config: &Configuration, check_config: &Option<Value>) -> Result<Option<CheckModule>,String> {
    if check_config.is_some() {
        let check_type = try_option!(check_config.unwrap().get("type"), "Check config contains no field 'type'");
        let module_paths = paths.for_module(config.name.as_str(), "check", &config.original_path, &config.store_path);

        let mut module = modules::check::get_module(sync_config.check.unwrap().check_type.as_str())?;

        module.init(config.name.as_str(), &check_config.unwrap(), module_paths, args.dry_run, args.no_docker);

        return Ok(Some(module));
    } else {
        return Ok(None);
    }
}

fn check_run(module: &Option<CheckModule>) -> Result<bool,String> {
    if module.is_some() {
        let result = module().unwrap().check()?;
        if result {
            debug!();
        } else {
            debug!();
        }
        return Ok(result);
    }

    return Ok(true);
}

fn check_update(module: &Option<CheckModule>) -> Result<(),String> {
    if module.is_some() {
        module.unwrap().update()?;
    }

    return Ok(());
}

fn check_clear(module: &mut Option<CheckModule>) -> Result<(),String> {
    if module.is_some() {
        module.unwrap().clear()?;
    }

    return Ok(());
}

pub fn list(args: &Arguments, paths: &Paths) -> Result<(),String> {

}