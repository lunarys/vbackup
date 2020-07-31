use crate::Arguments;
use crate::util::objects::time::{TimeFrameReference, TimeFrame, SaveData, TimeFrames};
use crate::util::objects::time::ExecutionTiming;
use crate::util::objects::paths::Paths;

use chrono::{DateTime, Local};
use crate::util::io::json;
use std::path::Path;
use std::collections::HashMap;
use std::rc::Rc;

// TODO: Log backup / debug

pub struct TimeframeChecker {
    force: bool,
    timeframes: HashMap<String, Rc<TimeFrame>>
}

#[derive(PartialEq, Eq)]
enum Type { Backup, Sync }

impl TimeframeChecker {
    pub fn new(paths: &Paths, args: &Arguments) -> Result<Self, String> {
        let mut timeframes = json::from_file::<TimeFrames>(Path::new(&paths.timeframes_file))?;
        let timeframes_rc = timeframes
            .drain()
            .map(|(key, value)| (key, Rc::new(value)))
            .collect();

        return Ok(Self {
            force: args.force,
            timeframes: timeframes_rc
        });
    }

    pub fn check_backup_timeframes(&self,
                                   config_name: &str,
                                   configured_timeframes: Vec<TimeFrameReference>,
                                   savedata: &SaveData) -> Vec<ExecutionTiming> {
        return self.check_timeframes(Type::Backup, config_name, configured_timeframes, savedata);
    }

    pub fn check_sync_timeframes(&self,
                                 config_name: &str,
                                 configured_timeframes: Vec<TimeFrameReference>,
                                 savedata: &SaveData) -> Vec<ExecutionTiming> {
        return self.check_timeframes(Type::Sync, config_name, configured_timeframes, savedata);
    }

    fn check_timeframes(&self,
                            run_type: Type,
                            config_name: &str,
                            configured_timeframes: Vec<TimeFrameReference>,
                            savedata: &SaveData) -> Vec<ExecutionTiming> {
        let run_type_str = match run_type {
            Type::Backup => "backup",
            Type::Sync => "sync"
        };

        // Prepare current timestamp (for consistency) and queue of timeframes for backup
        let current_time : DateTime<Local> = chrono::Local::now();
        let mut queue_executions: Vec<ExecutionTiming> = vec![];

        // Fill queue with timeframes to run backup for
        for timeframe_ref in configured_timeframes {

            // if amount of saves is zero just skip further checks
            if timeframe_ref.amount.eq(&usize::min_value()) {
                // min_value is 0
                warn!("Amount of saves in timeframe '{}' for '{}' {} is zero, this might never be executed", &timeframe_ref.frame, config_name, run_type_str);
                continue;
            }

            // Parse time frame data
            let timeframe_opt = self.timeframes.get(&timeframe_ref.frame);
            if timeframe_opt.is_none() {
                error!("Referenced timeframe '{}' for '{}' {} does not exist", &timeframe_ref.frame, config_name, run_type_str);
                continue;
            };
            let timeframe = timeframe_opt.unwrap();

            // Get last backup (option as there might not be a last one)
            let last_option = match run_type {
                Type::Backup => savedata.lastsave.get(&timeframe.identifier),
                Type::Sync => savedata.lastsync.get(&timeframe.identifier)
            };

            // Only actually do check if the run is not forced
            let mut do_run = true;
            if !self.force {
                // Try to compare timings to the last run
                if let Some(last) = last_option {

                    // Compare elapsed time since last backup and the configured timeframe
                    if last.timestamp + timeframe.interval < current_time.timestamp() {
                        // do sync
                        debug!("{} for '{}' is required in timeframe '{}' considering the interval only", run_type_str, config_name, timeframe_ref.frame.as_str());
                    } else {
                        // do not sync
                        info!("{} for '{}' is not executed in timeframe '{}' due to time constraints", run_type_str, config_name, timeframe_ref.frame.as_str());
                        do_run = false;
                    }

                    // run sync only after backup
                    if run_type == Type::Sync {
                        let backup_after_sync = savedata.lastsave.is_empty() || savedata.lastsave.values().any(|backup| backup.timestamp > last.timestamp );
                        if !backup_after_sync {
                            info!("Sync for '{}' is not executed as there is no new backup since the last sync", config_name);
                            do_run = false;
                        }
                    }
                } else {
                    // Probably the first backup in this timeframe, just do it
                    info!("This is probably the first {} run in timeframe '{}' for '{}', interval check is skipped", run_type_str, timeframe_ref.frame.as_str(), config_name);
                }
            }

            if do_run {
                queue_executions.push(ExecutionTiming {
                    time_frame_reference: timeframe_ref,
                    time_frame: timeframe.clone(),
                    last_run: last_option.map(|content| content.clone()), // TODO: just cloned
                    execution_time: current_time
                });
            }
        }

        if self.force {
            debug!("{} for '{}' is forced in all timeframes", run_type_str, config_name);
        }

        if queue_executions.is_empty() {
            info!("{} is not required for '{}' in any timeframe due to time constraints", run_type_str, config_name);
        }

        return queue_executions;
    }
}
