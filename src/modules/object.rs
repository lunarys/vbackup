use serde_json::Value;
use serde::{Deserialize,Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct Arguments {
    pub operation: String,
    pub dry_run: bool,
    pub verbose: bool,
    pub debug: bool,
    pub quiet: bool,
    pub force: bool,
    pub name: Option<String>,
    pub base_config: String,
    pub no_docker: bool
}

#[derive(Deserialize)]
pub struct Configuration {
    #[serde(default="default_bool_false")]
    pub disabled: bool,
    pub name: String,
    pub savedata_in_store: Option<bool>,
    pub original_path: Option<String>,
    pub store_path: Option<String>,
    pub backup: Option<BackupConfiguration>,
    pub sync: Option<SyncConfiguration>
}

#[derive(Deserialize)]
pub struct BackupConfiguration {
    #[serde(default="default_bool_false")]
    pub disabled: bool,
    #[serde(rename(deserialize = "type"))]
    pub backup_type: String,
    pub config: Value,
    pub check: Option<Value>,
    pub timeframes: Vec<TimeFrameReference>
}

#[derive(Deserialize)]
pub struct SyncConfiguration {
    #[serde(default="default_bool_false")]
    pub disabled: bool,
    #[serde(rename(deserialize = "type"))]
    pub sync_type: String,
    pub interval: TimeFrameReference,
    pub config: Value,
    pub check: Option<Value>,
    pub controller: Option<Value>
}

#[derive(Deserialize)]
pub struct TimeFrameReference {
    pub frame: String,
    #[serde(default="default_usize_1")]
    pub amount: usize
}

pub type TimeFrames = HashMap<String, TimeFrame>;

#[derive(Deserialize)]
pub struct TimeFrame {
    pub identifier: String,
    pub interval: i64,
}

fn default_usize_1() -> usize { 1 }
fn default_bool_false() -> bool { false }
fn default_bool_true() -> bool { true }

#[derive(Deserialize,Serialize)]
pub struct SaveData {
    pub lastsave: HashMap<String,TimeEntry>,
    pub nextsave: HashMap<String,TimeEntry>,
    pub lastsync: HashMap<String,TimeEntry>
}

#[derive(Deserialize,Serialize)]
pub struct TimeEntry {
    // TODO: Maybe also add key here with flatten thingy or so
    pub timestamp: i64,
    pub date: Option<String> // TODO: Is there a better data type?
}

#[derive(Deserialize)]
pub struct PathBase {
    #[serde(default="default_config_dir")]
    pub config_dir: String, // Here should be the configuration files
    #[serde(default="default_save_dir")]
    pub save_dir: String, // Default base directory for saves
    pub timeframes_file: Option<String>, // File containing timeframe definitions
    #[serde(default="default_tmp_dir")]
    pub tmp_dir: String, // Directory for temporary files
    pub auth_data_file: Option<String>, // File containing shared authentication information
    #[serde(default="default_bool_true")]
    pub savedata_in_store: bool,
    pub reporting_file: Option<String>,
    pub docker_images: Option<String>
}

fn default_config_dir() -> String { String::from("/etc/vbackup") }
fn default_save_dir() -> String { String::from("/var/vbackup") }
fn default_tmp_dir() -> String { String::from("/tmp/vbackup ")}

pub struct Paths {
    pub config_dir: String, // Here should be the configuration files
    pub save_dir: String, // Default base directory for saves
    pub timeframes_file: String, // File containing timeframe definitions
    pub tmp_dir: String, // Directory for temporary files
    pub auth_data_file: String, // File containing shared authentication information
    pub savedata_in_store: bool,
    pub reporting_file: String,
    pub docker_images: String
}

pub struct ModulePaths<'a> {
    pub base_paths: &'a Paths,
    pub save_data: String, // Savedata file
    pub original_path: Option<String>, // Path of the original directory to back up
    pub store_path: String, // Path for a local backup (or just path that will be synced)
    pub module_data_dir: String // Path for the modules to store additional data
}

impl Paths {
    pub fn from(base: PathBase) -> Paths {
        return Paths {
            timeframes_file: base.timeframes_file.unwrap_or(format!("{}/timeframes.json", &base.config_dir)),
            auth_data_file: base.auth_data_file.unwrap_or(format!("{}/auth_data.json", &base.config_dir)),
            reporting_file: base.reporting_file.unwrap_or(format!("{}/reporting.json", &base.config_dir)),
            docker_images: base.docker_images.unwrap_or(format!("{}/images", &base.config_dir)),
            config_dir: base.config_dir,
            save_dir: base.save_dir,
            tmp_dir: base.tmp_dir,
            savedata_in_store: base.savedata_in_store
        }
    }

    pub fn for_module(&self, name: &str, module_type: &str, original_path: &Option<String>, save_path_option: &Option<String>, savedata_in_store: &Option<bool>) -> ModulePaths {
        let backup_path = if save_path_option.is_some() {
            String::from(save_path_option.as_ref().unwrap())
        } else {
            if original_path.is_none() {
                warn!("Using default store path (to sync from) when no backup was configured");
            }
            format!("{}/{}", self.save_dir.as_str(), name)
        };

        let module_data_base = format!("{}/.module_data/{}", self.save_dir.as_str(), name);
        let module_data_dir = format!("{}/{}", module_data_base.as_str(), module_type);

        let save_data = if savedata_in_store.unwrap_or(self.savedata_in_store) {
            format!("{}/.savedata.json", backup_path.as_str())
        } else {
            format!("{}/savedata.json", module_data_base.as_str())
        };

        return ModulePaths {
            base_paths: self,
            save_data,
            original_path: original_path.clone(),
            store_path: backup_path,
            module_data_dir
        }
    }
}