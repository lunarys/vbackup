use std::path::Path;
use std::rc::Rc;
use crate::{Arguments, log_error, try_option, try_result};
use crate::modules::backup::{BackupModule, BackupWrapper};
use crate::modules::controller::bundle::BundleableControllerWrapper;
use crate::modules::controller::ControllerModule;
use crate::modules::sync::{SyncModule, SyncWrapper};
use crate::util::io::json;
use crate::util::io::user::ask_user_boolean;
use crate::util::objects::configuration::Configuration;
use crate::util::objects::paths::{ModulePaths, Paths};

pub fn main(args: Arguments, paths: Rc<Paths>) -> Result<(),String> {
    let (name, file_path) = if let Some(name) = args.name.as_ref() {
        (name, format!("{}/volumes/{}.json", &paths.config_dir, args.name.as_ref().unwrap()))
    } else {
        return Err(String::from("Please set an volume to restore, batch restore is not supported"));
    };

    info!("Running restore for '{}'", name);

    let config = json::from_file::<Configuration>(Path::new(&file_path))?;

    if config.disabled {
        warn!("Configuration is disabled...")
    }

    if let Some(sync_config) = config.sync.as_ref() {
        debug!("Checking sync configuration...");

        let confirmation = if config.disabled {
            warn!("Configuration is disabled...");
            ask_user_boolean("Run sync restore anyway?", true)?
        } else if sync_config.disabled {
            warn!("Sync configuration is disabled...");
            ask_user_boolean("Run sync restore anyway?", true)?
        } else {
            ask_user_boolean("Run sync restore?", true)?
        };

        if !confirmation {
            info!("Not running sync restore.");
        } else {
            debug!("Setting up sync module...");

            let module_paths = ModulePaths::for_sync_module(&paths, "sync", &config);
            let module = SyncModule::new(sync_config.sync_type.as_str(), name, &sync_config.config, module_paths, &args)?;

            debug!("Setting up controller...");

            // check for controller
            let controller = if let Some(controller_config) = sync_config.controller.as_ref() {
                let controller_type_opt = try_option!(controller_config.get("type"), "Controller config contains no field 'type'");
                let controller_type = try_option!(controller_type_opt.as_str(), "Could not get controller type as string");
                let module_paths = ModulePaths::for_sync_module(&paths, "controller", &config);

                let mut controller = ControllerModule::new(controller_type, config.name.as_str(), &controller_config, module_paths, &args)?;

                debug!("Starting controller init...");
                controller.as_mut_controller().init()?;
                info!("Starting controller...");
                let started = controller.as_mut_controller().begin()?;

                if started {
                    info!("Remote device is online.")
                } else {
                    let err = "Remote device is not online and/or can not be started.";
                    error!("{}", err);
                    return Err(String::from(err));
                }

                Some(controller)
            } else {
                debug!("No controller found.... Skipping.");
                None
            };

            info!("Starting sync restore...");

            let restore_result = module.restore();

            log_error!(&restore_result);

            // controller should be terminated regardless of sync restore result
            let result = if let Some(mut controller) = controller {
                debug!("Running controller end procedure...");
                let end_result = controller.as_mut_controller().end();
                log_error!(&end_result);

                if let Ok(ended) = end_result.as_ref() {
                    if !ended {
                        // anything we can do here?
                        warn!("Controller did not end properly...");
                    }
                }

                debug!("Running controller clear procedure...");
                let clear_result = controller.as_mut_controller().clear();
                log_error!(&clear_result);

                restore_result.and(end_result).and(clear_result)
            } else {
                restore_result
            };

            if result.is_err() {
                return result;
            }

            info!("Sync restore successful.")
        }
    } else {
        info!("No sync configuration found...");
    };

    if let Some(backup_config) = config.backup.as_ref() {
        debug!("Checking backup configuration");

        let confirmation = if config.disabled {
            warn!("Configuration is disabled...");
            ask_user_boolean("Run backup restore anyway?", true)?
        } else if backup_config.disabled {
            warn!("Backup configuration is disabled...");
            ask_user_boolean("Run backup restore anyway?", true)?
        } else {
            ask_user_boolean("Run backup restore?", true)?
        };

        if !confirmation {
            info!("Not running backup restore.");
        } else {
            let module_paths = ModulePaths::for_backup_module(&paths, "backup", &config);
            let module = BackupModule::new(backup_config.backup_type.as_str(), name, &backup_config.config, module_paths, &args)?;

            info!("Starting backup restore...");

            let restore_result = module.restore();

            try_result!(restore_result, "Backup restore failed...");
            info!("Backup restore successful");
        }
    } else {
        info!("No backup configuration found...");
    };

    return Ok(());
}