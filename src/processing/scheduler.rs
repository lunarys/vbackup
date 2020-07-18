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
    name: String,
    configurations: Vec<Configuration>,
    controller: Value
}

pub fn get_exec_order(config_list: Vec<Configuration>,
                      include_backup: bool,
                      include_sync: bool) -> Result<Vec<ConfigurationBundle>, String> {
    let mut sync_controller_bundle_map: HashMap<String, SyncControllerBundle> = HashMap::new();
    let mut backup_list = vec![];
    let mut sync_list = vec![];

    for configuration in config_list {
        if configuration.disabled {
            continue;
        }

        if let Some(backup) = &configuration.backup {
            if !backup.disabled && include_backup {
                backup_list.push(ConfigurationBundle::Backup(configuration.clone()));
            }
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
            }
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