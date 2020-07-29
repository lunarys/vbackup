use crate::Arguments;
use crate::modules::reporting::ReportingModule;
use crate::processing::scheduler::{SyncControllerBundle};
use crate::util::io::file;
use crate::util::objects::time::{TimeFrames,SaveData};
use crate::util::objects::paths::Paths;
use crate::modules::traits::Reporting;
use crate::processing::backup::backup;
use crate::processing::sync::sync;
use crate::modules::controller::ControllerModule;
use crate::modules::controller::bundle::ControllerBundle;

use crate::{log_error, try_option};
use crate::processing::scheduler::{ConfigurationBundle};
use crate::processing::preprocessor::{SyncUnit, BackupUnit};

pub fn process_configurations(args: &Arguments,
                              reporter: &ReportingModule,
                              configurations: Vec<ConfigurationBundle>) -> Result<(),String> {
    for configuration in configurations {
        let result = match configuration {
            ConfigurationBundle::Backup(backup) => {
                process_backup(&backup, args, reporter)
            },
            ConfigurationBundle::Sync(sync) => {
                process_sync(&sync, args, reporter, None)
            },
            ConfigurationBundle::SyncControllerBundle(sync_controller_bundle) => {
                process_sync_controller_bundle(&sync_controller_bundle, args, reporter)
            }
        };

        // If there was any error log it and go ahead
        log_error!(result);
    }

    return Ok(());
}

fn process_backup(config: &BackupUnit,
                  args: &Arguments,
                  reporter: &ReportingModule) -> Result<(), String> {
    // Save those paths for later, as the ModulePaths will be moved
    let original_path = config.module_paths.source.clone();
    let store_path = config.module_paths.destination.clone();

    // TODO: Pass paths by reference
    // Run the backup and report the result
    let result = backup(args, config.module_paths.clone(), config.config.as_ref(), config.backup_config.clone(), config.savedata, config.timeframes);
    result_reporter("backup", result, config.config.name.as_str(), reporter);

    // Calculate and report the size of the original files
    size_reporter("backup", "original", original_path.as_str(), config.config.name.as_str(), reporter, args);

    // Calculate and report the size of the backup files
    size_reporter("backup", "backup", store_path.as_str(), config.config.name.as_str(), reporter, args);

    return Ok(());
}

fn process_sync(config: &SyncUnit,
                args: &Arguments,
                reporter: &ReportingModule,
                controller_overwrite: Option<&ControllerModule>) -> Result<(), String> {
    // Save owned objects of configuration and path
    let store_path = config.module_paths.source.clone();

    // Run the sync and report the result
    // TODO: let result = sync(args, config.module_paths.clone(), config.config.as_ref(), config.sync_config.clone(), config.savedata.as_mut(), config.timeframe);
    let result = Err(String::from("Commented out"));
    result_reporter("sync", result, config.config.name.as_str(), reporter);

    // Calculate and report size of the synced files
    // TODO: Current implementation just takes the size of the local files...
    size_reporter("sync", "sync", store_path.as_str(), config.config.name.as_str(), reporter, args);

    return Ok(());
}

fn process_sync_controller_bundle(sync_controller_bundle: &SyncControllerBundle,
                                  args: &Arguments,
                                  reporter: &ReportingModule) -> Result<(), String> {
    // TODO: Create custom controller and then just process sync for every
    // let controller_bundle = ControllerBundle::new(args, paths, &sync_controller_bundle);
    // let controller = controller_bundle.wrap();

    //for configuration in sync_controller_bundle.configurations {
        // TODO: let result = process_sync(&configuration, args, paths, timeframes, savedata, reporter, Some(&controller));
        //log_error!(result);
    //}

    // TODO: Handle result
    // controller_bundle.done();

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