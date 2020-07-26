use crate::modules::object::{Configuration, Paths, Arguments};
use crate::util::io::{file, json};

use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

pub enum ConfigurationBundle {
    Backup(Configuration),
    Sync(Configuration),
    SyncControllerBundle(SyncControllerBundle)
}

pub struct SyncControllerBundle {
    pub name: String,
    pub configurations: Vec<Configuration>,
    pub controller: Value
}

pub fn get_exec_order(config_list: Vec<Configuration>,
                      include_backup: bool,
                      include_sync: bool) -> Result<Vec<ConfigurationBundle>, String> {
    let mut sync_controller_bundle_map: HashMap<String, SyncControllerBundle> = HashMap::new();
    let mut backup_list = vec![];
    let mut sync_list = vec![];

    for configuration in config_list {
        if configuration.disabled {
            info!("Configuration for '{}' is disabled", configuration.name.as_str());
            continue;
        }

        if let Some(backup) = &configuration.backup {
            if !backup.disabled && include_backup {
                backup_list.push(ConfigurationBundle::Backup(configuration.clone()));
            } else if backup.disabled {
                info!("Backup for '{}' is disabled", configuration.name.as_str());
            }
        } else {
            debug!("No backup configured for '{}'", configuration.name.as_str());
        }

        // TODO: various clone statements
        if let Some(sync) = &configuration.sync {
            if !sync.disabled && include_sync {
                if let Some(controller) = &sync.controller {
                    if let Some(bundle) = controller.get("bundle") {
                        // TODO: currently bundle_name is the only way to check for the same controllers...
                        if let Some(bundle_name) = bundle.as_str() {
                            if let Some(bundle_list) = sync_controller_bundle_map.get_mut(bundle_name) {
                                bundle_list.configurations.push(configuration.clone());
                            } else {
                                sync_controller_bundle_map.insert(String::from(bundle_name), SyncControllerBundle {
                                    name: format!("{}", bundle_name),
                                    configurations: vec![configuration.clone()],
                                    // controller configuration is assumed to be the same in a bundle
                                    controller: controller.clone()
                                });
                            }
                        } else {
                            warn!("Bundle name has to be provided as a string. Value is disregarded.");
                            sync_list.push(ConfigurationBundle::Sync(configuration.clone()));
                        }
                    } else {
                        sync_list.push(ConfigurationBundle::Sync(configuration.clone()));
                    }
                }
            } else if sync.disabled {
                info!("Sync for '{}' is disabled", configuration.name.as_str());
            }
        } else {
            debug!("No sync configured for '{}'", configuration.name.as_str());
        }
    };

    let mut configuration_list = vec![];
    configuration_list.append(&mut backup_list);
    configuration_list.append(&mut sync_list);
    configuration_list.append(&mut sync_controller_bundle_map
        .drain()
        .map(|(_,v)| ConfigurationBundle::SyncControllerBundle(v))
        .collect()
    );
    /*
    let readable_list = configuration_list.iter().map(|c| {
        match c {
            ConfigurationBundle::Backup(b) => "backup " + b.name.as_str(),
            ConfigurationBundle::Sync(s) => "sync " + s.name.as_str(),
            ConfigurationBundle::SyncControllerBundle(b) => {
                "sync (bundle)"
            }
        }
    }).collect()
*/
    return Ok(configuration_list);
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