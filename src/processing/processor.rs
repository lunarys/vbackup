use crate::Arguments;
use crate::modules::reporting::ReportingModule;
use crate::util::objects::savedata::{SaveDataCollection};
use crate::util::objects::reporting::{RunType,Status};
use crate::processing::backup::backup;
use crate::processing::sync::sync;
use crate::processing::preprocessor::{ConfigurationUnit, SyncControllerBundle, SyncUnit, BackupUnit};
use crate::processing::timeframe_check;
use crate::modules::controller::ControllerModule;

use crate::{log_error,try_result};

use core::borrow::{BorrowMut, Borrow};
use chrono::{DateTime, Local};
use crate::util::objects::configuration::StrategyConfiguration;
use crate::util::command::CommandWrapper;

pub fn process_configurations(args: &Arguments,
                              reporter: &ReportingModule,
                              configurations: Vec<ConfigurationUnit>,
                              mut savedata_collection: SaveDataCollection) -> Result<(),String> {
    for configuration in configurations {

        // TODO: Maybe execution time update should be improved
        let current_time : DateTime<Local> = chrono::Local::now();

        let result = match configuration {
            ConfigurationUnit::Backup(mut backup) => {
                backup.timeframes.iter_mut().for_each(|timeframe| {
                    timeframe.execution_time = current_time.clone();
                });
                process_backup(&mut backup, &mut savedata_collection, args, reporter)
            },
            ConfigurationUnit::Sync(mut sync) => {
                sync.timeframe.execution_time = current_time;
                process_sync(&mut sync, &mut savedata_collection, args, reporter, None)
            },
            ConfigurationUnit::SyncControllerBundle(mut sync_controller_bundle) => {
                sync_controller_bundle.units.iter_mut().for_each(|sync| {
                    sync.timeframe.execution_time = current_time.clone()
                });
                process_sync_controller_bundle(&mut sync_controller_bundle, &mut savedata_collection, args, reporter)
            }
        };

        // If there was any error log it and go ahead
        log_error!(result);
    }

    return Ok(());
}

fn process_backup(config: &mut BackupUnit,
                  savedata_collection: &mut SaveDataCollection,
                  args: &Arguments,
                  reporter: &ReportingModule) -> Result<(), String> {
    let savedata = savedata_collection
        .get_mut(config.config.name.as_str())
        .ok_or(format!("No savedata is present for '{}' backup", config.config.name.as_str()))?;

    // Announce that this backup is starting
    reporter.report_status(RunType::BACKUP, Some(config.config.name.clone()), Status::START);

    // run before
    let setup_result = run_before(config.backup_config.setup.as_ref(), args.dry_run);

    // TODO: Pass paths by reference
    // Run the backup and report the result
    let result = setup_result.and_then(|()| {
        backup(args, config, savedata)
    });
    result_reporter(RunType::BACKUP, result, config.config.name.borrow(), reporter);

    // run after
    try_result!(run_after(config.backup_config.setup.as_ref(), args.dry_run), "Script after backup failed");

    return Ok(());
}

fn process_sync(config: &mut SyncUnit,
                savedata_collection: &mut SaveDataCollection,
                args: &Arguments,
                reporter: &ReportingModule,
                controller_override: Option<&mut ControllerModule>) -> Result<(), String> {
    let savedata = savedata_collection
        .get_mut(config.config.name.as_str())
        .ok_or(format!("No savedata is present for '{}' backup", config.config.name.as_str()))?;

    if !args.force {
        if !timeframe_check::check_sync_after_backup(&config.timeframe, savedata, config.has_backup) {
            info!("Sync for '{}' is not executed as there is no new backup since the last sync", config.config.name.as_str());
            reporter.report_status(RunType::SYNC, Some(config.config.name.clone()), Status::SKIP);
            return Ok(());
        } else {
            debug!("Sync for '{}' is executed as there was a recent backup", config.config.name.as_str());
        }
    } else {
        debug!("Skipping check for sync after backup due to forced run");
    }

    // Announce that this sync is starting
    reporter.report_status(RunType::SYNC, Some(config.config.name.clone()), Status::START);

    // run before
    let setup_result = run_before(config.sync_config.setup.as_ref(), args.dry_run);

    // Run the sync and report the result (if before script was successful)
    let result = setup_result.and_then(|()| {
        sync(args, config, savedata, controller_override)
    });
    result_reporter(RunType::SYNC, result, config.config.name.borrow(), reporter);

    // run after
    try_result!(run_after(config.sync_config.setup.as_ref(), args.dry_run), "Script after sync failed");

    return Ok(());
}

fn process_sync_controller_bundle(sync_controller_bundle: &mut SyncControllerBundle,
                                  savedata: &mut SaveDataCollection,
                                  args: &Arguments,
                                  reporter: &ReportingModule) -> Result<(), String> {

    for configuration in &mut sync_controller_bundle.units {
        let result = process_sync(configuration, savedata, args, reporter, Some(sync_controller_bundle.controller.borrow_mut()));
        log_error!(result);
    }

    let result = match &mut sync_controller_bundle.controller {
        ControllerModule::Bundle(bundle) => bundle.done(),
        _ => {
            // Just constrain this for now
            Err(String::from("Expected controller bundle for bundled sync modules, got something else... Controller might not stop properly"))
        }
    };
    log_error!(result);

    return Ok(());
}

// ############################ Helper functions ############################
fn run_before(setup_opt: Option<&StrategyConfiguration>, dry_run: bool) -> Result<(), String> {
    if let Some(setup) = setup_opt {
        if let Some(before) = setup.before.as_ref() {
            for script in before {
                CommandWrapper::new_with_args("sh", vec!["-c", script]).run_configuration(false, dry_run)?;
            }
        }

        if let Some(containers) = setup.containers.as_ref() {
            let mut cmd_args = vec!["stop"];

            containers.iter().for_each(|container| {
                cmd_args.push(container);
            });

            CommandWrapper::new_with_args("docker", cmd_args).run_configuration(false, dry_run)?;
        }
    }

    Ok(())
}

fn run_after(setup_opt: Option<&StrategyConfiguration>, dry_run: bool) -> Result<(), String> {
    if let Some(setup) = setup_opt {
        if let Some(containers) = setup.containers.as_ref() {
            let mut cmd_args = vec!["start"];

            containers.iter().rev().for_each(|container| {
                cmd_args.push(container)
            });

            CommandWrapper::new_with_args("docker", cmd_args).run_configuration(false, dry_run)?;
        }

        if let Some(after) = setup.after.as_ref() {
            for script in after {
                CommandWrapper::new_with_args("sh", vec!["-c", script]).run_configuration(false, dry_run)?;
            }
        }
    }

    Ok(())
}

fn result_reporter(run_type: RunType,
                   result: Result<bool,String>,
                   config_name: &String,
                   reporter: &ReportingModule) {
    match result {
        Ok(true) => {
            info!("{} for '{}' was successfully executed", run_type, config_name);
            reporter.report_status(run_type, Some(config_name.clone()), Status::DONE);
        },
        Ok(false) => {
            info!("{} for '{}' was not executed", run_type, config_name);
            reporter.report_status(run_type, Some(config_name.clone()), Status::SKIP);
        },
        Err(err) => {
            error!("{} for '{}' failed: {}", run_type, config_name, err);
            reporter.report_status(run_type, Some(config_name.clone()), Status::ERROR);
        }
    }
}