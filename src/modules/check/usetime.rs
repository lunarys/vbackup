use crate::modules::traits::Check;
use crate::util::io::{json,file};
use crate::{try_result,dry_run};

use crate::util::objects::time::{ExecutionTiming};
use crate::util::objects::paths::{ModulePaths};
use crate::Arguments;

use std::path::Path;
use serde_json::{Value,json as create_json};
use serde::{Deserialize};

const USETIME_PROPERTY: &str = "usetime";

pub struct Usetime {
    config: Configuration,
    dry_run: bool
}

struct BackupInfo {
    usetime: i64,
    file_content: Option<FileContent>
}

enum FileContent {
    Json(Value),
    Plain(String)
}

#[derive(Deserialize)]
struct Configuration {
    #[serde(default="default_format_json")]
    json: bool,

    #[serde(default="relative_backup_info")]
    file: String,
    targeted_usetime: i64
}

fn default_format_json() -> bool { true }
fn relative_backup_info() -> String {
    return String::from("backupinfo/props.info");
}

impl Check for Usetime {
    const MODULE_NAME: &'static str = "usetime";

    fn new(_name: &str, config_json: &Value, _paths: ModulePaths, args: &Arguments) -> Result<Box<Self>, String> {
        let config = json::from_value::<Configuration>(config_json.clone())?; // TODO: - clone

        return Ok(Box::new(Self {
            config,
            dry_run: args.dry_run
        }));
    }

    fn init(&mut self) -> Result<(), String> {
        return Ok(());
    }

    fn check(&mut self, timing: &ExecutionTiming) -> Result<bool, String> {
        if timing.last_run.is_some() {
            let backup_info = self.read_backupinfo()?;
            let test_result = self.config.targeted_usetime < backup_info.usetime;

            if test_result {
                debug!("Usetime for server is larger than targeted usetime");
            } else {
                debug!("Usetime for server is smaller than targeted usetime");
            }

            return Ok(test_result);
        } else {
            debug!("There was no previous backup, additional check is not required");
            return Ok(true);
        }
    }

    fn update(&mut self, _timing: &ExecutionTiming) -> Result<(), String> {
        let mut backup_info = self.read_backupinfo()?;

        debug!("Resetting usetime for server to zero");

        if self.dry_run {
            dry_run!(format!("Writing usetime=0 to file '{}'", self.config.file));
            return Ok(());
        } else {
            return self.reset_backupinfo(&mut backup_info);
        }
    }

    fn clear(&mut self) -> Result<(), String> {
        return Ok(());
    }
}

impl Usetime {
    fn read_backupinfo(&self) -> Result<BackupInfo, String> {
        let file = self.config.file.as_str();

        let mut content_opt: Option<FileContent> = None;
        let mut usetime_opt: Option<i64> = None;

        if file::exists(file) {
            if self.config.json {
                let raw_content = json::from_file::<Value>(Path::new(file))?;

                if let Some(usetime_value) = raw_content.get(USETIME_PROPERTY) {
                    usetime_opt = usetime_value.as_i64();
                } else {
                    warn!("usetime property is not included in json");
                }

                content_opt = Some(FileContent::Json(raw_content));
            } else {
                let raw_content = file::read(file)?;

                for line in raw_content.split("\n") {
                    let separator_option = line.find("=");
                    if separator_option.is_none() {
                        continue;
                    } else {
                        let (key, value_tmp): (&str, &str) = line.split_at(separator_option.unwrap());
                        let value = if value_tmp.starts_with("=") {
                            let (_, tmp) = value_tmp.split_at(1);
                            tmp
                        } else {
                            value_tmp
                        };

                        if key.to_lowercase().as_str() == USETIME_PROPERTY {
                            usetime_opt = Some(try_result!(value.parse(), "Could not parse usetime for minecraft server"))
                        } else {
                            ()
                        }
                    }
                }

                content_opt = Some(FileContent::Plain(raw_content));
            }
        }

        if let Some(usetime) = usetime_opt {
            debug!("Read usetime: {}", usetime);
        } else {
            debug!("Usetime did not exist, assuming zero");
        }

        let result = BackupInfo {
            usetime: usetime_opt.unwrap_or(0),
            file_content: content_opt
        };

        return Ok(result);
    }

    fn reset_backupinfo(&self, info: &mut BackupInfo) -> Result<(), String> {
        if let Some(file_content) = info.file_content.as_mut() {
            let file = self.config.file.as_str();

            return match file_content {
                FileContent::Json(content) => {
                    if let Some(json_object) = content.as_object_mut() {
                        json_object.insert(String::from(USETIME_PROPERTY), create_json!(0));
                        json::to_file::<Value>(Path::new(file), content)
                    } else {
                        // usetime is not an object...
                        warn!("Usetime file did not contain an object, not updating");
                        Ok(())
                    }
                }
                FileContent::Plain(content) => {
                    // Use the original value to reset the usetime
                    let to_replace = format!("{}={}", USETIME_PROPERTY, info.usetime);
                    let replace_with = format!("{}=0", USETIME_PROPERTY);
                    let content = content.replace(to_replace.as_str(), replace_with.as_str());

                    file::write(file, content.as_str(), true)
                }
            }
        } else {
            trace!("Usetime file was empty, not updating");
        }

        Ok(())
    }
}