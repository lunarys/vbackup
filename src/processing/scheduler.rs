use crate::Arguments;
use crate::util::objects::configuration::Configuration;
use crate::util::objects::paths::Paths;
use crate::util::io::{file, json};
use crate::processing::preprocessor::{ConfigurationUnit,BackupUnit,SyncUnit};
use crate::modules::traits::Bundleable;
use crate::modules::controller::ControllerModule;
use crate::modules::controller::bundle::ControllerBundle;

use serde_json::Value;
use std::path::Path;

pub enum ConfigurationBundle {
    Backup(BackupUnit),
    Sync(SyncUnit),
    SyncControllerBundle(SyncControllerBundle)
}

pub struct SyncControllerBundle {
    pub units: Vec<SyncUnit>,
    pub controller: ControllerModule
}

struct SyncControllerBundleBuilder {
    units: Vec<SyncUnit>,
    main_controller: ControllerModule,
    additional_controllers: Vec<ControllerModule>
}

pub fn get_exec_order(config_list: Vec<ConfigurationUnit>) -> Result<Vec<ConfigurationBundle>, String> {
    let mut sync_controller_bundles: Vec<SyncControllerBundleBuilder> = vec![];
    let mut backup_list = vec![];
    let mut sync_list = vec![];

    for configuration in config_list {
        match configuration {
            ConfigurationUnit::Backup(backup) => {
                backup_list.push(ConfigurationBundle::Backup(backup))
            },
            ConfigurationUnit::Sync(mut sync) => {
                if let Some(controller) = sync.controller.take() {
                    if controller.can_bundle() {
                        if let Some(index) = sync_controller_bundles.iter().position(|x| controller.can_bundle_with(&x.main_controller)) {
                            sync_controller_bundles[index].units.push(sync);
                            sync_controller_bundles[index].additional_controllers.push(controller);
                        } else {
                            sync_controller_bundles.push(SyncControllerBundleBuilder {
                                units: vec![sync],
                                main_controller: controller,
                                additional_controllers: vec![]
                            });
                        }
                    } else {
                        sync_list.push(ConfigurationBundle::Sync(sync))
                    }
                } else {
                    sync_list.push(ConfigurationBundle::Sync(sync))
                }
            }
        }
    }

    let mut configuration_list = vec![];

    // First backups
    configuration_list.append(backup_list.as_mut());

    // Then sync bundles
    for mut bundle_builder in sync_controller_bundles {
        if bundle_builder.additional_controllers.is_empty() {
            if let Some(mut sync) = bundle_builder.units.pop() {
                let mut controller = bundle_builder.main_controller;
                controller.init_single();
                sync.controller = Some(controller);
                sync_list.push(ConfigurationBundle::Sync(sync));
            } else {
                return Err(String::from("SyncUnit in scheduler missing for unbundled Bundleable"));
            }
        } else {
            let controller_bundle = ControllerBundle::new(
                bundle_builder.main_controller,
                bundle_builder.additional_controllers
            )?.wrap();

            configuration_list.push(ConfigurationBundle::SyncControllerBundle(SyncControllerBundle {
                units: bundle_builder.units,
                controller: controller_bundle
            }));
        }
    }

    // Then syncs
    configuration_list.append( sync_list.as_mut());

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