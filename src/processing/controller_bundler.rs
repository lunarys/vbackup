use crate::util::objects::paths::{Paths, ModulePaths};
use crate::util::objects::reporting::{RunType,Status};
use crate::modules::controller::{ControllerModule, BundleableWrapper, ControllerWrapper};
use crate::modules::controller::bundle::ControllerBundle;
use crate::modules::reporting::ReportingModule;
use crate::processing::preprocessor::{ConfigurationUnit, SyncControllerBundle, SyncUnit};
use crate::Arguments;

use crate::{try_option};

use std::rc::Rc;
use std::collections::HashMap;
use core::borrow::{BorrowMut};

struct SyncControllerBundleBuilder {
    units: Vec<SyncUnit>,
    controller: ControllerBundle
}

pub fn load_controllers(mut configurations: Vec<ConfigurationUnit>, args: &Arguments, paths: &Rc<Paths>, reporter: &mut ReportingModule) -> Vec<ConfigurationUnit> {
    let mut done = vec![];
    let mut bundle_types: HashMap<String,Vec<SyncControllerBundleBuilder>> = HashMap::new();

    configurations
        .drain(..)
        .for_each(|configuration| {
            match configuration {
                ConfigurationUnit::Backup(backup) => {
                    done.push(ConfigurationUnit::Backup(backup));
                },
                ConfigurationUnit::SyncControllerBundle(sync) => {
                    // There are no bundles at this point, so just keep it as is for now
                    done.push(ConfigurationUnit::SyncControllerBundle(sync))
                },
                ConfigurationUnit::Sync(sync) => {
                    let name = sync.config.name.clone();
                    let result = handle_controller_bundle(sync, done.borrow_mut(), bundle_types.borrow_mut(), paths, args);

                    if let Err(err) = result {
                        error!("Could not load controller for '{}', skipping this sync configuration: {}", &name, err);
                        report_error(reporter, RunType::SYNC, &name);
                    }
                }
            }
        });

    bundle_types.values_mut().for_each(|value_type| {
        value_type.drain(..).for_each(|mut builder| {
            if builder.units.is_empty() {
                error!("Controller bundle builder for sync does not have any sync units");
            } else if builder.units.len() == 1 {
                let mut unit: SyncUnit = builder.units.pop().unwrap();
                match builder.controller.into_simple_controller() {
                    Ok(controller) => {
                        unit.controller = Some(controller);
                        done.push(ConfigurationUnit::Sync(unit));
                    },
                    Err(err) => {
                        error!("Could not get simple controller from controller bundle with single sync unit for '{}': {}... Skipping sync", unit.config.name.as_str(), err);
                    }
                }
            } else {
                let bundled_config_names = builder.units.iter().map(|unit| unit.config.name.as_str()).collect::<Vec<&str>>().join(", ");
                debug!("Bundled {} controllers for: {}", builder.controller.get_module_name(), bundled_config_names);

                done.push(ConfigurationUnit::SyncControllerBundle(SyncControllerBundle {
                    units: builder.units,
                    controller: builder.controller.into_controller()
                }));
            }
        })
    });

    return done;
}

fn handle_controller_bundle(mut sync: SyncUnit, done: &mut Vec<ConfigurationUnit>, bundles: &mut HashMap<String,Vec<SyncControllerBundleBuilder>>, paths: &Rc<Paths>, args: &Arguments) -> Result<(),String> {
    if let Some(controller_config) = sync.sync_config.controller.clone() { // TODO: clone ?
        let controller_type_opt = try_option!(controller_config.get("type"), "Controller config contains no field 'type'");
        let controller_type = try_option!(controller_type_opt.as_str(), "Could not get controller type as string");
        let module_paths = ModulePaths::for_sync_module(paths, "controller", &sync.config);

        if !ControllerBundle::can_bundle_type(controller_type) {
            // If there is no option to bundle, just finish it off as is

            sync.controller = Some(ControllerModule::new(controller_type, sync.config.name.as_str(), &controller_config, module_paths, args)?);
            done.push(ConfigurationUnit::Sync(sync));
        } else {
            if let Some(entry_list) = bundles.get_mut(controller_type) {
                // If there is another controller of the same type, try to find a fitting one for bundling

                let pos_opt = entry_list.iter_mut().position(|entry| {
                    let result = entry.controller.try_bundle(sync.config.name.as_str(), &controller_config);
                    match result {
                        Ok(result) => result,
                        Err(err) => {
                            error!("Could not bundle controllers for '{}': {}", sync.config.name.as_str(), err);
                            false
                        }
                    }
                });

                if let Some(pos) = pos_opt {
                    entry_list.get_mut(pos).unwrap().units.push(sync);
                } else {
                    entry_list.push(SyncControllerBundleBuilder {
                        controller: ControllerBundle::new(controller_type, sync.config.name.as_str(), &controller_config, paths, args)?,
                        units: vec![sync]
                    });
                }
            } else {
                let controller = ControllerBundle::new(controller_type, sync.config.name.as_str(), &controller_config, paths, args)?;

                bundles.insert(String::from(controller_type), vec![SyncControllerBundleBuilder {
                    units: vec![sync],
                    controller
                }]);
            }
        }
    } else {
        done.push(ConfigurationUnit::Sync(sync));
    }

    return Ok(());
}

fn report_error(reporter: &mut ReportingModule, run_type: RunType, name: &String) {
    reporter.report_status(run_type, Some(name.clone()), Status::ERROR);
}