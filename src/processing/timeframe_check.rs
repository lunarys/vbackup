use crate::Arguments;
use crate::util::objects::time::{TimeFrameReference, TimeFrame, TimeEntry, SaveData, TimeFrames};
use crate::modules::check::CheckModule;
use crate::util::objects::time::ExecutionTiming;
use crate::util::helper::{check as check_helper};
use crate::util::objects::paths::Paths;

use chrono::{DateTime, Local};
use crate::util::io::json;
use std::path::Path;
use std::collections::HashMap;
use std::rc::Rc;


// TODO: Log backup / debug
// TODO: does check for sync work differently?

pub struct TimeframeChecker {
    force: bool,
    timeframes: HashMap<String, Rc<TimeFrame>>
}

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

    pub fn check_timeframes(&self,
                            configured_timeframes: Vec<TimeFrameReference>,
                            savedata: &SaveData) -> Vec<ExecutionTiming> {
        // Prepare current timestamp (for consistency) and queue of timeframes for backup
        let config_name = String::from("TODO: insert name");
        let current_time : DateTime<Local> = chrono::Local::now();
        let mut queue_executions: Vec<ExecutionTiming> = vec![];

        // Fill queue with timeframes to run backup for
        for timeframe_ref in configured_timeframes {

            // if amount of saves is zero just skip further checks
            if timeframe_ref.amount.eq(&usize::min_value()) {
                // min_value is 0
                warn!("Amount of saves in timeframe '{}' for '{}' backup is zero, no backup will be created", &timeframe_ref.frame, config_name.as_str());
                continue;
            }

            // Parse time frame data
            let timeframe_opt = self.timeframes.get(&timeframe_ref.frame);
            if timeframe_opt.is_none() {
                error!("Referenced timeframe '{}' for '{}' backup does not exist", &timeframe_ref.frame, config_name.as_str());
                continue;
            };
            let timeframe = timeframe_opt.unwrap();

            // Get last backup (option as there might not be a last one)
            let last_backup_option = savedata.lastsave.get(&timeframe.identifier);

            // Only actually do check if the run is not forced
            let mut do_backup = true;
            if !self.force {
                // Try to compare timings to the last run
                if last_backup_option.is_some() {

                    // Compare elapsed time since last backup and the configured timeframe
                    if last_backup_option.unwrap().timestamp + timeframe.interval < current_time.timestamp() {
                        // do sync
                        debug!("Backup for '{}' is required in timeframe '{}' considering the interval only", config_name.as_str(), timeframe_ref.frame.as_str());
                    } else {
                        // do not sync
                        info!("Backup for '{}' is not executed in timeframe '{}' due to the interval", config_name.as_str(), timeframe_ref.frame.as_str());
                        do_backup = false;
                    }
                } else {

                    // Probably the first backup in this timeframe, just do it
                    info!("This is probably the first backup run in timeframe '{}' for '{}', interval check is skipped", timeframe_ref.frame.as_str(), config_name.as_str());
                }
            } else {
                debug!("Run in timeframe '{}' is forced", timeframe_ref.frame.as_str())
            }

            if do_backup {
                queue_executions.push(ExecutionTiming {
                    time_frame_reference: timeframe_ref,
                    time_frame: timeframe.clone(),
                    time_entry: last_backup_option.map(|content| content.clone()), // TODO: just cloned
                    execution_time: current_time
                });
            }
        }

        return queue_executions;
    }
}
