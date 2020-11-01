use crate::Arguments;
use crate::modules::reporting::ReportingModule;
use crate::util::io::file;
use crate::util::objects::savedata::{SaveDataCollection};
use crate::util::objects::reporting::{SizeType,RunType,Status};
use crate::util::objects::paths::SourcePath;
use crate::processing::backup::backup;
use crate::processing::sync::sync;
use crate::processing::preprocessor::{ConfigurationUnit, SyncControllerBundle, SyncUnit, BackupUnit};
use crate::processing::timeframe_check;
use crate::modules::controller::ControllerModule;

use crate::{log_error};

use core::borrow::{BorrowMut, Borrow};
use chrono::{DateTime, Local};

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
    // Save those paths for later, as the ModulePaths will be moved
    let original_path = config.module_paths.source.clone();
    let store_path = config.module_paths.destination.clone();

    let savedata = savedata_collection
        .get_mut(config.config.name.as_str())
        .ok_or(format!("No savedata is present for '{}' backup", config.config.name.as_str()))?;

    // Announce that this backup is starting
    reporter.report_status(RunType::BACKUP, Some(config.config.name.clone()), Status::START);

    // TODO: Pass paths by reference
    // Run the backup and report the result
    let result = backup(args, config, savedata);
    result_reporter(RunType::BACKUP, result, config.config.name.borrow(), reporter);

    // Calculate and report the size of the original files
    size_reporter(RunType::BACKUP, SizeType::ORIGINAL, original_path.borrow(), config.config.name.borrow(), reporter, args);

    // Calculate and report the size of the backup files
    size_reporter(RunType::BACKUP, SizeType::BACKUP, &SourcePath::Single(store_path.clone()), config.config.name.borrow(), reporter, args);

    return Ok(());
}

fn process_sync(config: &mut SyncUnit,
                savedata_collection: &mut SaveDataCollection,
                args: &Arguments,
                reporter: &ReportingModule,
                controller_override: Option<&mut ControllerModule>) -> Result<(), String> {
    // Save owned objects of configuration and path
    let store_path = config.module_paths.source.clone();

    let savedata = savedata_collection
        .get_mut(config.config.name.as_str())
        .ok_or(format!("No savedata is present for '{}' backup", config.config.name.as_str()))?;

    if !args.force {
        if !timeframe_check::check_sync_after_backup(&config.timeframe, savedata, config.has_backup) {
            info!("Sync for '{}' is not executed as there is no new backup since the last sync", config.config.name.as_str());
            reporter.report_status(RunType::SYNC, Some(config.config.name.clone()), Status::SKIP);
            return Ok(());
        }
    } else {
        debug!("Skipping check for sync after backup due to forced run");
    }

    // Announce that this sync is starting
    reporter.report_status(RunType::SYNC, Some(config.config.name.clone()), Status::START);

    // Run the sync and report the result
    let result = sync(args, config, savedata, controller_override);
    result_reporter(RunType::SYNC, result, config.config.name.borrow(), reporter);

    // Calculate and report size of the synced files
    // TODO: Current implementation just takes the size of the local files...
    size_reporter(RunType::SYNC, SizeType::SYNC, store_path.borrow(), config.config.name.borrow(), reporter, args);

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

fn size_reporter(run_type: RunType,
                 directory_type: SizeType,
                 path: &SourcePath,
                 config_name: &String,
                 reporter: &ReportingModule,
                 args: &Arguments) {
    match file::size(path, args.no_docker) {
        Ok(curr_size) => {
            reporter.report_size(run_type, directory_type, Some(config_name.clone()), curr_size);
        },
        Err(err) => {
            error!("Could not read size of the {} files: {}", directory_type, if args.dry_run { "This is likely due to this being a dry-run" } else { err.as_str() });
        }
    }
}
