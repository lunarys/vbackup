use crate::modules::object::{Configuration, BackupConfiguration, SyncConfiguration, ModulePaths, Paths, Arguments};
use crate::modules::check::{CheckModule, Reference};
use crate::modules::controller::ControllerModule;
use crate::util::io::savefile::get_savedata;
use crate::util::helper::{controller as controller_helper};
use crate::util::helper::{check as check_helper};
use crate::util::objects::time::{ExecutionTiming, SaveData, TimeFrames};
use crate::processing::timeframe_check;

use std::rc::Rc;
use chrono::{DateTime, Local};
use std::borrow::Borrow;

pub enum ConfigurationUnit {
    Backup(BackupUnit),
    Sync(SyncUnit)
}

enum ConfigurationUnitBuilder {
    Backup(BackupUnitBuilder),
    Sync(SyncUnitBuilder)
}

struct ConfigurationSplit {
    config: Configuration,
    backup_config: Option<BackupConfiguration>,
    backup_paths: Option<ModulePaths>,
    sync_config: Option<SyncConfiguration>,
    sync_paths: Option<ModulePaths>,
    savedata: Option<Rc<SaveData>>
}

pub struct BackupUnit {
    pub config: Rc<Configuration>,
    pub backup_config: BackupConfiguration,
    pub check: Option<CheckModule>,
    pub module_paths: ModulePaths,
    pub savedata: Rc<SaveData>,
    pub timeframes: Vec<ExecutionTiming>
}

struct BackupUnitBuilder {
    config: Rc<Configuration>,
    backup_config: BackupConfiguration,
    check: Option<CheckModule>,
    module_paths: ModulePaths,
    savedata: Rc<SaveData>,
    timeframes: Option<Vec<ExecutionTiming>>
}

pub struct SyncUnit {
    pub config: Rc<Configuration>,
    pub sync_config: SyncConfiguration,
    pub check: Option<CheckModule>,
    pub controller: Option<ControllerModule>,
    pub module_paths: ModulePaths,
    pub savedata: Rc<SaveData>,
    pub timeframe: ExecutionTiming
}

struct SyncUnitBuilder {
    config: Rc<Configuration>,
    sync_config: SyncConfiguration,
    check: Option<CheckModule>,
    controller: Option<ControllerModule>,
    module_paths: ModulePaths,
    savedata: Rc<SaveData>,
    timeframes: Option<Vec<ExecutionTiming>>
}

pub fn preprocess(configurations: Vec<Configuration>, args: &Arguments, paths: &Rc<Paths>) -> Result<Vec<ConfigurationUnit>,String> {
    let without_disabled = filter_disabled(configurations);
    let with_module_paths = load_module_paths(without_disabled, paths);
    let with_savedata = load_savedata(with_module_paths);
    let split = flatten_processing_list(with_savedata);
    let with_time_constraints = filter_time_constraints(split, args, paths)?;
    let with_checks = load_checks(with_time_constraints, args, paths);
    let with_additional_check = filter_additional_check(with_checks, args);
    let with_controllers = load_controllers(with_additional_check, args, paths);

    return Ok(assemble_from_builders(with_controllers));
}

fn filter_disabled(mut configurations: Vec<Configuration>) -> Vec<ConfigurationSplit> {
    // step 1
    //  filter disabled
    //  move to split
    return configurations.drain(..)
        .filter(|config| !config.disabled)
        .map(|mut config| {
            ConfigurationSplit {
                backup_config: config.backup.clone().filter(|backup| !backup.disabled),
                sync_config: config.sync.clone().filter(|sync| !sync.disabled),
                config,
                backup_paths: None,
                sync_paths: None,
                savedata: None
            }
        })
        .collect();
}

fn load_module_paths(mut configurations: Vec<ConfigurationSplit>, paths: &Rc<Paths>) -> Vec<ConfigurationSplit> {
    // step 2
    //  load module paths
    for mut configuration in &mut configurations {
        configuration.backup_paths = Some(ModulePaths::for_backup_module(paths, "backup", &configuration.config));
        configuration.sync_paths = Some(ModulePaths::for_sync_module(paths, "sync", &configuration.config));
    }

    return configurations;
}

fn load_savedata(mut configurations: Vec<ConfigurationSplit>) -> Vec<ConfigurationSplit> {
    // step 3
    //  load savedata for all
    return configurations
        .drain(..)
        .filter_map(|mut config| {
            if config.backup_paths.is_none() {
                error!("Module paths for '{}' not loaded in preprocessor... skipping configuration", config.config.name.as_str());
                return None;
            }

            // the save_data path is the same for both modules (backup and sync)
            let savedata_result = get_savedata(config.backup_paths.as_ref().unwrap().save_data.as_str());
            let savedata = match savedata_result {
                Ok(savedata) => savedata,
                Err(err) => {
                    error!("Could not read savedata for '{}': {}", config.config.name.as_str(), err);
                    return None;
                }
            };

            config.savedata = Some(Rc::new(savedata));
            return Some(config);
        })
        .collect();
}

fn flatten_processing_list(mut configurations: Vec<ConfigurationSplit>) -> Vec<ConfigurationUnitBuilder> {
    // step 4
    let mut result = vec![];
    configurations
        .drain(..)
        .for_each(|mut config| {
            let config_rc = Rc::new(config.config);
            if config.savedata.is_none() {
                error!("Savedata for '{}' is missing... skipping potential runs", config_rc.name.as_str());
                return;
            }

            let savedata_rc = config.savedata.unwrap();

            if let Some(backup_config) = config.backup_config.take() {
                result.push(ConfigurationUnitBuilder::Backup(BackupUnitBuilder {
                    config: config_rc.clone(),
                    backup_config,
                    savedata: savedata_rc.clone(),
                    check: None,
                    module_paths: config.backup_paths.unwrap(),
                    timeframes: None
                }))
            }

            if let Some(sync_config) = config.sync_config.take() {
                result.push(ConfigurationUnitBuilder::Sync(SyncUnitBuilder {
                    config: config_rc.clone(),
                    sync_config,
                    savedata: savedata_rc.clone(),
                    check: None,
                    controller: None,
                    module_paths: config.sync_paths.unwrap(),
                    timeframes: None
                }))
            }
        });

    return result;
}

fn filter_time_constraints(mut configurations: Vec<ConfigurationUnitBuilder>, args: &Arguments, paths: &Rc<Paths>) -> Result<Vec<ConfigurationUnitBuilder>,String> {
    // step 5
    if args.force {
        debug!("Skipping time constraints checks due to forced run");
        return Ok(configurations);
    }

    let timeframe_checker = timeframe_check::TimeframeChecker::new(paths, args)?;

    let result = configurations
        .drain(..)
        .filter_map(|configuration| {
            let mut timeframes = match &configuration {
                ConfigurationUnitBuilder::Backup(backup) => {
                    timeframe_checker.check_timeframes(backup.backup_config.timeframes.clone(), backup.savedata.as_ref())
                },
                ConfigurationUnitBuilder::Sync(sync) => {
                    timeframe_checker.check_timeframes(vec![sync.sync_config.interval.clone()], sync.savedata.as_ref())
                }
            };

            if timeframes.is_empty() {
                return None;
            }

            // TODO: Probably possible to do this more elegantly
            let new_configuration = match configuration {
                ConfigurationUnitBuilder::Backup(mut backup) => {
                    backup.timeframes = Some(timeframes);
                    ConfigurationUnitBuilder::Backup(backup)
                },
                ConfigurationUnitBuilder::Sync(mut sync) => {
                    sync.timeframes = Some(timeframes);
                    ConfigurationUnitBuilder::Sync(sync)
                }
            };

            return Some(new_configuration);
        })
        .collect();

    return Ok(result);
}

fn load_checks(mut configurations: Vec<ConfigurationUnitBuilder>, args: &Arguments, paths: &Rc<Paths>) -> Vec<ConfigurationUnitBuilder> {
    // step 6
    return configurations
        .drain(..)
        .filter_map(|configuration| {
            match configuration {
                ConfigurationUnitBuilder::Backup(mut backup) => {
                    let check_result = check_helper::init(args, paths, &backup.config, &backup.backup_config.check, Reference::Backup);
                    match check_result {
                        Ok(result) => {
                            backup.check = result;
                            return Some(ConfigurationUnitBuilder::Backup(backup));
                        },
                        Err(err) => {
                            // TODO: Might want to remove sync also if this fails
                            error!("Could not load check for '{}', skipping this backup configuration: {}", backup.config.name.as_str(), err);
                            return None;
                        }
                    }
                },
                ConfigurationUnitBuilder::Sync(mut sync) => {
                    let check_result = check_helper::init(args, paths, &sync.config, &sync.sync_config.check, Reference::Sync);
                    match check_result {
                        Ok(result) => {
                            sync.check = result;
                            return Some(ConfigurationUnitBuilder::Sync(sync));
                        },
                        Err(err) => {
                            error!("Could not load check for '{}', skipping this sync configuration: {}", sync.config.name.as_str(), err);
                            return None;
                        }
                    }
                }
            }
        })
        .collect();
}

fn filter_additional_check(mut configurations: Vec<ConfigurationUnitBuilder>, args: &Arguments) -> Vec<ConfigurationUnitBuilder> {
    // step 7
    if args.force {
        debug!("Skipping additional checks due to forced run");
        return configurations;
    }

    return configurations
        .drain(..)
        .filter_map(|configuration| {
            fn filter_timeframes(run_type: &str, name: &str, check: &Option<CheckModule>, timeframes: Option<Vec<ExecutionTiming>>) -> Option<Vec<ExecutionTiming>> {
                if check.is_none() {
                    debug!("There is no additional check for '{}' {}, only using the interval checks", name, run_type);
                    return timeframes;
                }

                let filtered_timeframes = if let Some(mut timeframes) = timeframes {
                    timeframes.drain(..).filter(|timeframe| {
                        let result = check_helper::run(check, &timeframe.execution_time, timeframe.time_frame.as_ref(), &timeframe.time_entry.as_ref());
                        match result {
                            Ok(success) => success,
                            Err(err) => {
                                error!("Additional check for '{}' {} failed... skipping run", name, run_type);
                                return false;
                            }
                        }
                    }).collect()
                } else {
                    error!("Timeframes not loaded for '{}' {}, even though they should be... skipping run", name, run_type);
                    vec![]
                };

                return Some(filtered_timeframes);
            }

            match configuration {
                ConfigurationUnitBuilder::Backup(mut backup) => {
                    backup.timeframes = filter_timeframes("backup", backup.config.name.as_str(), &backup.check, backup.timeframes)
                        .filter(|some| !some.is_empty());

                    if backup.timeframes.is_none() {
                        return None;
                    } else {
                        return Some(ConfigurationUnitBuilder::Backup(backup));
                    }
                },
                ConfigurationUnitBuilder::Sync(mut sync) => {
                    sync.timeframes = filter_timeframes("sync", sync.config.name.as_str(), &sync.check, sync.timeframes)
                        .filter(|some| !some.is_empty());

                    if sync.timeframes.is_none() {
                        return None;
                    } else {
                        return Some(ConfigurationUnitBuilder::Sync(sync));
                    }
                }
            }

            /* TODO
            if filtered_timeframes.is_empty() {
                info!("{} for '{}' is not executed in timeframe '{}' due to the additional check", run_type, name, timeframe_ref.frame.as_str());
                return false;
            } else {
                debug!("{} for '{}' is required in timeframe '{}' considering the additional check", run_type, name, timeframe_ref.frame.as_str());
                return true;
            }
            */
        })
        .collect();
}

fn load_controllers(mut configurations: Vec<ConfigurationUnitBuilder>, args: &Arguments, paths: &Rc<Paths>) -> Vec<ConfigurationUnitBuilder> {
    // step 8
    return configurations
        .drain(..)
        .filter_map(|configuration| {
            if let ConfigurationUnitBuilder::Sync(mut sync) = configuration {
                let controller_result = controller_helper::init(args, paths, sync.config.as_ref(), &sync.sync_config.controller.as_ref());
                match controller_result {
                    Ok(result) => {
                        sync.controller = result;
                        return Some(ConfigurationUnitBuilder::Sync(sync));
                    },
                    Err(err) => {
                        error!("Could not load controller for '{}', skipping this sync configuration: {}", sync.config.name.as_str(), err);
                        return None;
                    }
                }
            } else {
                return Some(configuration);
            }
        })
        .collect();
}

fn assemble_from_builders(mut configurations: Vec<ConfigurationUnitBuilder>) -> Vec<ConfigurationUnit> {
    return configurations
        .drain(..)
        .filter_map(|configuration| {
            match configuration {
                ConfigurationUnitBuilder::Backup(backup_builder) => {
                    let timeframes_option = backup_builder.timeframes
                        .filter(|l| !l.is_empty());

                    if timeframes_option.is_none() {
                        error!("Backup for '{}' does not have any timeframes, skipping run", backup_builder.config.name.as_str());
                        return None;
                    }

                    Some(ConfigurationUnit::Backup(BackupUnit {
                        config: backup_builder.config,
                        backup_config: backup_builder.backup_config,
                        check: backup_builder.check,
                        module_paths: backup_builder.module_paths,
                        savedata: backup_builder.savedata,
                        timeframes: timeframes_option.unwrap()
                    }))
                },
                ConfigurationUnitBuilder::Sync(sync_builder) => {
                    let timeframe_option = sync_builder.timeframes
                        .filter(|l| l.len() == 1)
                        .map(|mut l| l.pop().unwrap());

                    if timeframe_option.is_none() {
                        error!("Sync for '{}' does not have exactly one timeframe, skipping run", sync_builder.config.name.as_str());
                        return None;
                    }

                    Some(ConfigurationUnit::Sync(SyncUnit {
                        config: sync_builder.config,
                        sync_config: sync_builder.sync_config,
                        check: sync_builder.check,
                        controller: sync_builder.controller,
                        module_paths: sync_builder.module_paths,
                        savedata: sync_builder.savedata,
                        timeframe: timeframe_option.unwrap()
                    }))
                }
            }
        })
        .collect();
}