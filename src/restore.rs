use std::path::Path;
use std::rc::Rc;
use crate::{Arguments, try_result};
use crate::modules::backup::{BackupModule, BackupWrapper};
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
            let module_paths = ModulePaths::for_sync_module(&paths, "sync", &config);
            let module = SyncModule::new(sync_config.sync_type.as_str(), name, &sync_config.config, module_paths, &args)?;

            info!("Starting sync restore...");

            let restore_result = module.restore();

            try_result!(restore_result, "Sync restore failed...");
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