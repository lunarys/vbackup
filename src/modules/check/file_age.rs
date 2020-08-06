use crate::modules::traits::Check;
use crate::{try_result,try_option,dry_run};
use crate::util::command::CommandWrapper;
use crate::util::objects::time::{ExecutionTiming};
use crate::util::objects::paths::{ModulePaths};
use crate::Arguments;

use serde_json::Value;

pub struct FileAge {
    paths: ModulePaths,
    no_docker: bool,
    dry_run: bool
}

impl Check for FileAge {
    const MODULE_NAME: &'static str = "file-age";

    fn new(_name: &str, _config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Box<Self>, String> {
        // TODO: Could include an ignore list in _config_json

        return Ok(Box::new(Self {
            paths,
            no_docker: args.no_docker,
            dry_run: args.dry_run
        }))
    }

    fn init(&mut self) -> Result<(), String> {
        return Ok(());
    }

    // TODO: cache result to not run the whole check for every timeframe
    fn check(&self, frame: &ExecutionTiming) -> Result<bool, String> {
        let last_run = if frame.last_run.is_none() {
            // If there is no last run, just run it
            debug!("Check is not necessary as there was no run before");
            return Ok(true);
        } else {
            trace!("Checking the age of files before doing anything");
            frame.last_run.as_ref().unwrap()
        };

        let check_path = &self.paths.source;

        let mut command_base = if self.no_docker {
            let mut command = CommandWrapper::new("sh");
            command.arg_str("-c");
            command
        } else {
            let mut command = CommandWrapper::new("docker");
            command.arg_str("run")
                .arg_str("--rm")
                .arg_str("--name=vbackup-check-fileage-tmp")
                .arg_string(format!("--volume={}:/volume", check_path))
                .arg_str("alpine")
                .arg_str("sh")
                .arg_str("-c");
            command
        };

        let search_path = if self.no_docker {
            check_path.as_str()
        } else {
            "/volume"
        };

        let command_actual = format!("[[ -d '{s}' ]] && [[ ! -z \"$(ls -A '{s}')\" ]] && find {s} -type f -print0 | xargs -0 stat -c '%Y;%n' | grep -v -e .savedata.json | sort -nr | head -n 1", s = search_path);
        command_base.arg_string(command_actual);

        if self.dry_run {
            dry_run!(command_base.to_string());
        }

        let output = command_base.run_get_output()?;
        let split_pos: usize = try_option!(output.find(";"), "Expected semicolon for split of check output");
        let (timestamp_str,filename) = output.split_at(split_pos);

        trace!("Newest file is '{}' and was changed at '{}'", filename, timestamp_str);

        let file_timestamp: i64 = try_result!(timestamp_str.parse(), "Could not parse timestamp from string");

        if last_run.timestamp < file_timestamp {
            // A file was written after last run
            debug!("Newest file is newer that last run, run now");
            return Ok(true);
        } else {
            // No file was written after last run
            debug!("Newest file is older than last run, do not run now");
            return Ok(false);
        }
    }

    fn update(&mut self, _frame: &ExecutionTiming) -> Result<(), String> {
        // This check is stateless, so no update is required
        return Ok(())
    }

    fn clear(&mut self) -> Result<(), String> {
        return Ok(());
    }
}