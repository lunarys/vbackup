use crate::modules::traits::Check;
use crate::{try_result,try_option,dry_run};
use crate::util::command::CommandWrapper;
use crate::util::objects::time::{ExecutionTiming};
use crate::util::objects::paths::{ModulePaths,SourcePath};
use crate::util::io::json;
use crate::Arguments;

use serde_json::Value;
use serde::{Deserialize};
use crate::util::docker;

pub struct FileAge {
    paths: ModulePaths,
    no_docker: bool,
    dry_run: bool,
    config: Configuration
}

#[derive(Deserialize)]
struct Configuration {
    exclude: Option<Vec<String>>
}

impl Check for FileAge {
    const MODULE_NAME: &'static str = "file-age";

    fn new(_name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Box<Self>, String> {
        let config = json::from_value(config_json.clone())?; // TODO: - clone

        return Ok(Box::new(Self {
            paths,
            no_docker: args.no_docker,
            dry_run: args.dry_run,
            config
        }))
    }

    fn init(&mut self) -> Result<(), String> {
        // Build local docker image
        if !self.no_docker {
            docker::build_image_if_missing(&self.paths.base_paths, "file-age.Dockerfile", "vbackup-file-age")?;
        }

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

        let search_path = if self.no_docker {
            if let SourcePath::Single(path) = check_path {
                path.as_str()
            } else {
                return Err(String::from("Multiple source paths are not supported in file age check module without docker"));
            }
        } else {
            "/volume"
        };

        // a string of exclude options for grep to filter modified files
        let exclude_string = self.config.exclude.as_ref().map(|exclude_list| {
            exclude_list.iter()
                .map(|exclude_part| format!("-e '{}'", exclude_part))
                .collect::<Vec<String>>()
                .join(" ")
        });

        // generate the command to execute from its individual parts
        let precondition_check = format!("[[ -d '{s}' ]] && [[ ! -z \"$(ls -A '{s}')\" ]]", s = search_path);
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

        if self.dry_run {
            dry_run!(command_base.to_string());
        }

        let output = command_base.run_get_output()?;
        let split_pos: usize = try_option!(output.find(";"), "Expected semicolon for split of check output");
        let (timestamp_str_fraction,filename) = output.split_at(split_pos);

        // timestamp contains a fraction (after a dot)
        let fraction_split_pos = timestamp_str_fraction.find('.');
        let timestamp_str = if let Some(pos) = fraction_split_pos {
            timestamp_str_fraction.split_at(pos).0
        } else {
            // there is no dot, assume there is no fraction
            timestamp_str_fraction
        };

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