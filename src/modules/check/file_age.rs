use crate::modules::traits::Check;
use crate::{try_result,try_option,dry_run};
use crate::util::command::CommandWrapper;
use crate::util::objects::time::{ExecutionTiming};
use crate::util::objects::paths::{ModulePaths,SourcePath};
use crate::util::io::json;
use crate::util::docker;
use crate::Arguments;

use serde_json::Value;
use serde::{Deserialize};
use std::cmp::max;
use std::rc::Rc;

pub struct FileAge {
    paths: ModulePaths,
    args: Rc<Arguments>,
    config: Configuration,
    cached_result: Option<i64>,
    had_error: bool
}

#[derive(Deserialize)]
struct Configuration {
    exclude: Option<Vec<String>>
}

impl Check for FileAge {
    const MODULE_NAME: &'static str = "file-age";

    fn new(_name: &str, config_json: &Value, paths: ModulePaths, args: &Rc<Arguments>) -> Result<Box<Self>, String> {
        let config = json::from_value(config_json.clone())?; // TODO: - clone

        return Ok(Box::new(Self {
            paths,
            args: args.clone(),
            config,
            cached_result: None,
            had_error: false
        }))
    }

    fn init(&mut self) -> Result<(), String> {
        // Build local docker image
        if !self.args.no_docker {
            docker::build_image_if_missing(&self.paths.base_paths, "file-age.Dockerfile", "vbackup-file-age")?;
        }

        return Ok(());
    }

    fn check(&mut self, frame: &ExecutionTiming) -> Result<bool, String> {
        let last_run = if frame.last_run.is_none() {
            // If there is no last run, just run it
            debug!("Check is not necessary as there was no run before");
            return Ok(true);
        } else if self.had_error {
            return Err(String::from("There was an error in a previous file-age check, not trying again..."));
        } else {
            trace!("Checking the age of files before doing anything");
            frame.last_run.as_ref().unwrap()
        };

        if self.cached_result.is_none() {
            let check_path = &self.paths.source;

            let search_paths = if self.args.no_docker {
                match check_path {
                    SourcePath::Single(path) => {
                        vec![path.as_str()]
                    }
                    SourcePath::Multiple(paths) => {
                        paths.iter().map(|mapping| {mapping.path.as_str()}).collect()
                    }
                }
            } else {
                vec!["/volume"]
            };

            // a string of exclude options for grep to filter modified files
            let exclude_string = self.config.exclude.as_ref().map(|exclude_list| {
                exclude_list.iter()
                    .map(|exclude_part| format!("-e '{}'", exclude_part))
                    .collect::<Vec<String>>()
                    .join(" ")
            });

            let timestamp_result: Result<i64,String> = search_paths.iter().fold(Ok(0), |previous_result, search_path| {
                if previous_result.is_err() {
                    // just pass on the error if there was some...
                    return previous_result;
                }

                let mut command_base = if self.args.no_docker {
                    let mut command = CommandWrapper::new("bash");
                    command.arg_str("-c");
                    command
                } else {
                    let mut command = CommandWrapper::new("docker");
                    command.arg_str("run")
                        .arg_str("--rm")
                        .arg_str("--name=vbackup-check-fileage-tmp")
                        .add_docker_volume_mapping(check_path, "volume")
                        .arg_str("vbackup-file-age")
                        .arg_str("sh")
                        .arg_str("-c");
                    command
                };

                // generate the command to execute from its individual parts
                let precondition_check = format!("[ -e '{s}' ]", s = search_path);
                let generate_list = format!("find {s} -type f -printf '%T@;./%P\\n'", s = search_path);
                let filter_list = format!("grep -v -F -e .savedata.json {e}", e = exclude_string.as_deref().unwrap_or(""));
                let sort_list = "sort -nr";
                let get_result = "head -n 1";
                let command_actual = format!("{} && {} | {} | {} | {}",
                                             precondition_check,
                                             generate_list,
                                             filter_list,
                                             sort_list,
                                             get_result
                );
                command_base.arg_string(command_actual);

                if self.args.dry_run {
                    dry_run!(command_base.to_string());
                }

                let output = command_base.run_get_output()?;
                let split_pos: usize;
                if output == "" {
                    debug!("There appears to be no file in the check path '{}'", search_path);
                    return previous_result;
                } else {
                    split_pos = try_option!(output.find(";"), "Expected semicolon for split of check output");
                }
                let (timestamp_str_fraction,filename) = output.split_at(split_pos);

                // timestamp contains a fraction (after a dot)
                let fraction_split_pos = timestamp_str_fraction.find('.');
                let timestamp_str = if let Some(pos) = fraction_split_pos {
                    timestamp_str_fraction.split_at(pos).0
                } else {
                    // there is no dot, assume there is no fraction
                    timestamp_str_fraction
                };

                trace!("Newest file in '{}' is '{}' and was changed at '{}'", search_path, filename, timestamp_str);
                let current_timestamp: i64 = try_result!(timestamp_str.parse(), "Could not parse timestamp from string");

                previous_result.map(|previous_timestamp| {max(current_timestamp, previous_timestamp)})
            });

            match timestamp_result {
                Ok(timestamp) => {
                    trace!("Newest file in the given paths was changed at '{}'", timestamp);
                    self.cached_result = Some(timestamp);
                }
                Err(error) => {
                    error!("{}", error);
                    self.had_error = true;
                    return Err(String::from("Could not get the timestamp of the latest file editing"));
                }
            };
        } else {
            trace!("Using cached value from previous timeframe for file-age check");
        }

        return if last_run.timestamp < self.cached_result.unwrap() {
            // A file was written after last run
            debug!("Newest file is newer that last run, run now");
            Ok(true)
        } else {
            // No file was written after last run
            debug!("Newest file is older than last run, do not run now");
            Ok(false)
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