use crate::Arguments;
use crate::modules::check::{CheckModule, Reference};
use crate::modules::controller::ControllerModule;
use crate::modules::reporting::ReportingModule;
use crate::util::io::savefile::get_savedata;
use crate::util::helper::{check as check_helper};
use crate::util::objects::time::ExecutionTiming;
use crate::util::objects::savedata::SaveDataCollection;
use crate::util::objects::configuration::{Configuration, BackupConfiguration, SyncConfiguration};
use crate::util::objects::paths::{ModulePaths, Paths};
use crate::util::objects::reporting::{RunType,Status};
use crate::processing::{timeframe_check,controller_bundler};

use std::rc::Rc;
use core::borrow::Borrow;
use std::borrow::BorrowMut;

pub enum ConfigurationUnit {
    Backup(BackupUnit),
    Sync(SyncUnit),
    SyncControllerBundle(SyncControllerBundle)
}

pub struct SyncControllerBundle {
    pub units: Vec<SyncUnit>,
    pub controller: ControllerModule
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
    sync_paths: Option<ModulePaths>
}

pub struct BackupUnit {
    pub config: Rc<Configuration>,
    pub backup_config: BackupConfiguration,
    pub check: Option<CheckModule>,
    pub module_paths: ModulePaths,
    pub timeframes: Vec<ExecutionTiming>,
    pub has_sync: bool
}

struct BackupUnitBuilder {
    config: Rc<Configuration>,
    backup_config: BackupConfiguration,
    check: Option<CheckModule>,
    module_paths: ModulePaths,
    timeframes: Option<Vec<ExecutionTiming>>,
    has_sync: bool
}

pub struct SyncUnit {
    pub config: Rc<Configuration>,
    pub sync_config: SyncConfiguration,
    pub check: Option<CheckModule>,
    pub controller: Option<ControllerModule>,
    pub module_paths: ModulePaths,
    pub timeframe: ExecutionTiming,
    pub has_backup: bool
}

struct SyncUnitBuilder {
    config: Rc<Configuration>,
    sync_config: SyncConfiguration,
    check: Option<CheckModule>,
    module_paths: ModulePaths,
    timeframes: Option<Vec<ExecutionTiming>>,
    has_backup: bool
}

pub struct PreprocessorResult {
    pub configurations: Vec<ConfigurationUnit>,
    pub savedata: SaveDataCollection
}

pub fn preprocess(configurations: Vec<Configuration>,
                  args: &Arguments,
                  paths: &Rc<Paths>,
                  reporter: &ReportingModule,
                  do_backup: bool,
                  do_sync: bool) -> Result<PreprocessorResult,String> {
    if !do_backup && !do_sync {
        return Err(String::from("Preprocessor called for neither backup nor sync"));
    }

    let without_disabled = filter_disabled(configurations, reporter, args);
    let with_setup = load_default_setup_strategy(without_disabled);
    let with_module_paths = load_module_paths(with_setup, paths);
    let savedata = load_savedata(&with_module_paths, reporter);
    let split = flatten_processing_list(with_module_paths, do_backup, do_sync);
    let with_time_constraints = filter_time_constraints(split, args, paths, &savedata, reporter)?;
    let with_checks = load_checks(with_time_constraints, args, paths, reporter);
    let with_additional_check = filter_additional_check(with_checks, args, reporter);

    let assembled = assemble_from_builders(with_additional_check, reporter);
    let with_controllers = controller_bundler::load_controllers(assembled, args, paths, reporter);

    return Ok(PreprocessorResult {
        configurations: with_controllers,
        savedata
    });
}

fn filter_disabled(mut configurations: Vec<Configuration>, reporter: &ReportingModule, args: &Arguments) -> Vec<ConfigurationSplit> {
    // step 1
    //  filter disabled
    //  move to split
    return configurations.drain(..)
        .filter(|config| {
            if config.disabled {
                if args.override_disabled {
                    warn!("Configuration for '{}' is disabled, but will be executed due to the override argument", config.name.as_str());
                    return true;
                } else {
                    info!("Configuration for '{}' is disabled, skipping run", config.name.as_str());
                    reporter.report_status(RunType::RUN, Some(config.name.clone()), Status::DISABLED);
                }
            }

            return !config.disabled;
        })
        .map(|config| {
            ConfigurationSplit {
                backup_config: config.backup.clone().filter(|backup| {
                    if backup.disabled {
                        if args.override_disabled {
                            warn!("Backup for '{}' is disabled, but will be executed due to the override argument", config.name.as_str());
                            return true;
                        } else {
                            info!("Backup for '{}' is disabled, skipping run", config.name.as_str());
                            reporter.report_status(RunType::BACKUP, Some(config.name.clone()), Status::DISABLED);
                        }
                    }

                    return !backup.disabled;
                }),
                sync_config: config.sync.clone().filter(|sync| {
                    if sync.disabled {
                        if args.override_disabled {
                            warn!("Sync for '{}' is disabled, but will be executed due to the override argument", config.name.as_str());
                            return true;
                        } else {
                            info!("Sync for '{}' is disabled, skipping run", config.name.as_str());
                            reporter.report_status(RunType::SYNC, Some(config.name.clone()), Status::DISABLED);
                        }
                    }

                    return !sync.disabled;
                }),
                config,
                backup_paths: None,
                sync_paths: None
            }
        })
        .filter(|config| {
            config.backup_config.is_some() || config.sync_config.is_some()
        })
        .collect();
}

fn load_default_setup_strategy(mut configurations: Vec<ConfigurationSplit>) -> Vec<ConfigurationSplit> {
    // step 1.5
    //  if the configuration has a default setup copy it to the respective backup or sync part
    //  if a backup configuration exists use the backup there, otherwise use it for sync
    return configurations.drain(..)
        .map(|mut config| {
            if let Some(setup_config) = config.config.setup.as_ref() {
                if let Some(backup_config) = config.backup_config.as_mut() {
                    if backup_config.setup.is_none() {
                        backup_config.setup.replace(setup_config.clone());
                    }
                } else if let Some(sync_config) = config.sync_config.as_mut() {
                    if sync_config.setup.is_none() {
                        sync_config.setup.replace(setup_config.clone());
                    }
                }
            }

            return config;
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

fn load_savedata(configurations: &Vec<ConfigurationSplit>, reporter: &ReportingModule) -> SaveDataCollection {
    // step 3
    //  load savedata for all
    return configurations
        .iter()
        .filter_map(|config| {
            if config.backup_paths.is_none() {
                error!("Module paths for '{}' not loaded in preprocessor... skipping configuration", config.config.name.as_str());
                report_error(reporter, RunType::RUN, config.config.name.borrow());
                return None;
            }

            // the save_data path is the same for both modules (backup and sync)
            let savedata_result = get_savedata(config.backup_paths.as_ref().unwrap().save_data.as_str());
            let savedata = match savedata_result {
                Ok(savedata) => savedata,
                Err(err) => {
                    error!("Could not read savedata for '{}': {}", config.config.name.as_str(), err);
                    report_error(reporter, RunType::RUN, config.config.name.borrow());
                    return None;
                }
            };

            return Some((config.config.name.clone(), savedata));
        })
        .collect();
}

fn flatten_processing_list(mut configurations: Vec<ConfigurationSplit>, do_backup: bool, do_sync: bool) -> Vec<ConfigurationUnitBuilder> {
    // step 4
    let mut result = vec![];
    configurations
        .drain(..)
        .for_each(|mut config| {
            let config_rc = Rc::new(config.config);

            let has_sync = config.sync_config.is_some();
            let has_backup = config.backup_config.is_some();

            if do_backup {
                if let Some(backup_config) = config.backup_config.take() {
                    result.push(ConfigurationUnitBuilder::Backup(BackupUnitBuilder {
                        config: config_rc.clone(),
                        backup_config,
                        check: None,
                        module_paths: config.backup_paths.unwrap(),
                        timeframes: None,
                        has_sync
                    }))
                }
            }

            if do_sync {
                if let Some(sync_config) = config.sync_config.take() {
                    result.push(ConfigurationUnitBuilder::Sync(SyncUnitBuilder {
                        config: config_rc.clone(),
                        sync_config,
                        check: None,
                        module_paths: config.sync_paths.unwrap(),
                        timeframes: None,
                        has_backup
                    }))
                }
            }
        });

    return result;
}

fn filter_time_constraints(mut configurations: Vec<ConfigurationUnitBuilder>,
                           args: &Arguments,
                           paths: &Rc<Paths>,
                           savedata_collection: &SaveDataCollection,
                           reporter: &ReportingModule) -> Result<Vec<ConfigurationUnitBuilder>,String> {
    // step 5
    if args.force {
        info!("Skipping time constraints checks due to forced run");
        // still needs to run to load the timeframes
    }

    let timeframe_checker = timeframe_check::TimeframeChecker::new(paths, args)?;

    let result = configurations
        .drain(..)
        .filter_map(|configuration| {
            let (name,run_type) = match &configuration {
                ConfigurationUnitBuilder::Backup(backup) => (backup.config.name.borrow(),RunType::BACKUP),
                ConfigurationUnitBuilder::Sync(sync) => (sync.config.name.borrow(),RunType::SYNC)
            };

            let savedata = if let Some(savedata) = savedata_collection.get(name) {
                savedata
            } else {
                error!("Savedata not loaded for '{}' in time constraint filter", name);
                report_error(reporter, run_type, name);
                return None;
            };

            let timeframes = match &configuration {
                ConfigurationUnitBuilder::Backup(backup) => {
                    timeframe_checker.check_backup_timeframes(backup.config.name.as_str(), backup.backup_config.timeframes.clone(), savedata)
                },
                ConfigurationUnitBuilder::Sync(sync) => {
                    timeframe_checker.check_sync_timeframes(sync.config.name.as_str(), vec![sync.sync_config.interval.clone()], savedata)
                }
            };

            if timeframes.is_empty() {
                debug!("{} for '{}' is not executed due to time constraints", run_type, name);
                report_skip(reporter, run_type, name);
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

fn load_checks(mut configurations: Vec<ConfigurationUnitBuilder>,
               args: &Arguments,
               paths: &Rc<Paths>,
               reporter: &ReportingModule) -> Vec<ConfigurationUnitBuilder> {
    // step 6
    return configurations
        .drain(..)
        .filter_map(|configuration| {
            match configuration {
                ConfigurationUnitBuilder::Backup(mut backup) => {
                    let check_result = check_helper::init(args, paths, &backup.config, &backup.backup_config.check, Reference::Backup);
                    match check_result {
                        Ok(result) => {
                            if result.is_none() {
                                debug!("There is no additional check for the backup of '{}', only using the interval checks", backup.config.name.as_str());
                            }

                            backup.check = result;
                            return Some(ConfigurationUnitBuilder::Backup(backup));
                        },
                        Err(err) => {
                            error!("Could not load check for '{}', skipping this backup configuration: {}", backup.config.name.as_str(), err);
                            report_error(reporter, RunType::BACKUP, backup.config.name.borrow());
                            return None;
                        }
                    }
                },
                ConfigurationUnitBuilder::Sync(mut sync) => {
                    let check_result = check_helper::init(args, paths, &sync.config, &sync.sync_config.check, Reference::Sync);
                    match check_result {
                        Ok(result) => {
                            if result.is_none() {
                                debug!("There is no additional check for the sync of '{}', only using the interval checks", sync.config.name.as_str());
                            }

                            sync.check = result;
                            return Some(ConfigurationUnitBuilder::Sync(sync));
                        },
                        Err(err) => {
                            error!("Could not load check for '{}', skipping this sync configuration: {}", sync.config.name.as_str(), err);
                            report_error(reporter, RunType::SYNC, sync.config.name.borrow());
                            return None;
                        }
                    }
                }
            }
        })
        .collect();
}

fn filter_additional_check(mut configurations: Vec<ConfigurationUnitBuilder>, args: &Arguments, reporter: &ReportingModule) -> Vec<ConfigurationUnitBuilder> {
    // step 7
    if args.force {
        debug!("Skipping additional checks due to forced run");
        return configurations;
    }

    return configurations
        .drain(..)
        .filter_map(|configuration| {
            fn filter_timeframes(run_type: RunType, name: &String, check: &mut Option<CheckModule>, timeframes: Option<Vec<ExecutionTiming>>, reporter: &ReportingModule) -> Option<Vec<ExecutionTiming>> {
                if check.is_none() {
                    debug!("There is no additional check for '{}' {}, only using the interval checks", name, run_type);
                    return timeframes;
                }

                let filtered_timeframes = if let Some(mut timeframes) = timeframes {
                    timeframes.drain(..).filter(|timeframe| {
                        let result = check_helper::run(check, &timeframe);
                        match result {
                            Ok(success) => {
                                if success {
                                    debug!("{} for '{}' is required in timeframe '{}' considering the additional check", run_type, name, timeframe.time_frame_reference.frame.as_str());
                                } else {
                                    info!("{} for '{}' is not executed in timeframe '{}' due to the additional check", run_type, name, timeframe.time_frame_reference.frame.as_str());
                                }

                                return success;
                            },
                            Err(err) => {
                                error!("Additional check for '{}' {} failed... skipping run ({})", name, run_type, err);
                                report_error(reporter, run_type.clone(), name);
                                return false;
                            }
                        }
                    }).collect()
                } else {
                    error!("Timeframes not loaded for '{}' {}, even though they should be... skipping run", name, run_type);
                    report_error(reporter, run_type.clone(), name);
                    vec![]
                };

                if filtered_timeframes.is_empty() {
                    info!("{} for '{}' is not required in any timeframe due to additional check", run_type, name);
                    report_skip(reporter, run_type.clone(), name);
                    return None;
                } else {
                    return Some(filtered_timeframes);
                }
            }

            match configuration {
                ConfigurationUnitBuilder::Backup(mut backup) => {
                    backup.timeframes = filter_timeframes(
                        RunType::BACKUP,
                        backup.config.name.borrow(),
                        backup.check.borrow_mut(),
                        backup.timeframes,
                        reporter
                    );

                    if backup.timeframes.is_none() {
                        return None;
                    } else {
                        return Some(ConfigurationUnitBuilder::Backup(backup));
                    }
                },
                ConfigurationUnitBuilder::Sync(mut sync) => {
                    sync.timeframes = filter_timeframes(
                        RunType::SYNC,
                        sync.config.name.borrow(),
                        sync.check.borrow_mut(),
                        sync.timeframes,
                        reporter
                    );

                    if sync.timeframes.is_none() {
                        return None;
                    } else {
                        return Some(ConfigurationUnitBuilder::Sync(sync));
                    }
                }
            }
        })
        .collect();
}

fn assemble_from_builders(mut configurations: Vec<ConfigurationUnitBuilder>, reporter: &ReportingModule) -> Vec<ConfigurationUnit> {
    // step 8
    return configurations
        .drain(..)
        .filter_map(|configuration| {
            match configuration {
                ConfigurationUnitBuilder::Backup(backup_builder) => {
                    let timeframes_option = backup_builder.timeframes
                        .filter(|l| !l.is_empty());

                    if timeframes_option.is_none() {
                        error!("Backup for '{}' does not have any timeframes, skipping run", backup_builder.config.name.as_str());
                        report_error(reporter, RunType::BACKUP, backup_builder.config.name.borrow());
                        return None;
                    }

                    Some(ConfigurationUnit::Backup(BackupUnit {
                        config: backup_builder.config,
                        backup_config: backup_builder.backup_config,
                        check: backup_builder.check,
                        module_paths: backup_builder.module_paths,
                        timeframes: timeframes_option.unwrap(),
                        has_sync: backup_builder.has_sync
                    }))
                },
                ConfigurationUnitBuilder::Sync(sync_builder) => {
                    let timeframe_option = sync_builder.timeframes
                        .filter(|l| l.len() == 1)
                        .map(|mut l| l.pop().unwrap());

                    if timeframe_option.is_none() {
                        error!("Sync for '{}' does not have exactly one timeframe, skipping run", sync_builder.config.name.as_str());
                        report_error(reporter, RunType::SYNC, sync_builder.config.name.borrow());
                        return None;
                    }

                    Some(ConfigurationUnit::Sync(SyncUnit {
                        config: sync_builder.config,
                        sync_config: sync_builder.sync_config,
                        check: sync_builder.check,
                        controller: None,
                        module_paths: sync_builder.module_paths,
                        timeframe: timeframe_option.unwrap(),
                        has_backup: sync_builder.has_backup
                    }))
                }
            }
        })
        .collect();
}

fn report_skip(reporter: &ReportingModule, run_type: RunType, name: &String) {
    reporter.report_status(run_type, Some(name.clone()), Status::SKIP);
}

fn report_error(reporter: &ReportingModule, run_type: RunType, name: &String) {
    reporter.report_status(run_type, Some(name.clone()), Status::ERROR);
}