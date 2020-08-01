use crate::Arguments;
use crate::modules::reporting::ReportingModule;
use crate::processing::scheduler::{SyncControllerBundle};
use crate::util::io::file;
use crate::util::objects::time::{SaveDataCollection};
use crate::modules::traits::{Reporting};
use crate::processing::backup::backup;
use crate::processing::sync::sync;
use crate::processing::scheduler::{ConfigurationBundle};
use crate::processing::preprocessor::{SyncUnit, BackupUnit};
use crate::processing::timeframe_check;
use crate::modules::controller::ControllerModule;

use crate::{log_error};

use core::borrow::BorrowMut;
use chrono::{DateTime, Local};

pub fn process_configurations(args: &Arguments,
                              reporter: &ReportingModule,
                              configurations: Vec<ConfigurationBundle>,
                              mut savedata_collection: SaveDataCollection) -> Result<(),String> {
    for configuration in configurations {

        // TODO: Maybe execution time update should be improved
        let current_time : DateTime<Local> = chrono::Local::now();

        let result = match configuration {
            ConfigurationBundle::Backup(mut backup) => {
                backup.timeframes.iter_mut().for_each(|timeframe| {
                    timeframe.execution_time = current_time.clone();
                });
                process_backup(&mut backup, &mut savedata_collection, args, reporter)
            },
            ConfigurationBundle::Sync(mut sync) => {
                sync.timeframe.execution_time = current_time;
                process_sync(&mut sync, &mut savedata_collection, args, reporter, None)
            },
            ConfigurationBundle::SyncControllerBundle(mut sync_controller_bundle) => {
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
    log_error!(reporter.report(Some(&["backup", config.config.name.as_str()]), "starting"));

    // TODO: Pass paths by reference
    // Run the backup and report the result
    let result = backup(args, config, savedata);
    result_reporter("backup", result, config.config.name.as_str(), reporter);

    // Calculate and report the size of the original files
    size_reporter("backup", "original", original_path.as_str(), config.config.name.as_str(), reporter, args);

    // Calculate and report the size of the backup files
    size_reporter("backup", "backup", store_path.as_str(), config.config.name.as_str(), reporter, args);

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

    if !timeframe_check::check_sync_after_backup(&config.timeframe, savedata, config.has_backup) {
        info!("Sync for '{}' is not executed as there is no new backup since the last sync", config.config.name.as_str());
        log_error!(reporter.report(Some(&["sync", config.config.name.as_str()]), "skipped"));
        return Ok(());
    }

    // Announce that this sync is starting
    log_error!(reporter.report(Some(&["sync", config.config.name.as_str()]), "starting"));

    // Run the sync and report the result
    let result = sync(args, config, savedata, controller_override);
    result_reporter("sync", result, config.config.name.as_str(), reporter);

    // Calculate and report size of the synced files
    // TODO: Current implementation just takes the size of the local files...
    size_reporter("sync", "sync", store_path.as_str(), config.config.name.as_str(), reporter, args);

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
            // Just constraint this for now
            Err(String::from("Expected controller bundle for bundled sync modules, got something else... Controller might not stop properly"))
        }
    };
    log_error!(result);

    return Ok(());
}

// ############################ Helper functions ############################
fn result_reporter(run_type: &str,
                   result: Result<bool,String>,
                   config_name: &str,
                   reporter: &ReportingModule) {
    match result {
        Ok(true) => {
            info!("{} for '{}' was successfully executed", run_type, config_name);
            let report_result = reporter.report(Some(&[run_type, config_name]), "success");
            log_error!(report_result);
        },
        Ok(false) => {
            info!("{} for '{}' was not executed due to constraints", run_type, config_name);
            let report_result = reporter.report(Some(&[run_type, config_name]), "skipped");
            log_error!(report_result);
        },
        Err(err) => {
            error!("{} for '{}' failed: {}", run_type, config_name, err);
            let report_result = reporter.report(Some(&[run_type, config_name]), "failed");
            log_error!(report_result);
        }
    }
}

fn size_reporter(run_type: &str,
                 directory_type: &str,
                 path: &str,
                 config_name: &str,
                 reporter: &ReportingModule,
                 args: &Arguments) {
    match file::size(path, args.no_docker) {
        Ok(curr_size) => {
            log_error!(reporter.report(Some(&[run_type, config_name, "size", directory_type]), curr_size.to_string().as_str()));
        },
        Err(err) => {
            error!("Could not read size of the {} files: {}", directory_type, if args.dry_run { "This is likely due to this being a dry-run" } else { err.as_str() });
        }
    }
}
