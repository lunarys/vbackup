use serde_json::{Value};
use serde::{Deserialize};

#[derive(Deserialize)]
pub struct Arguments {
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
    pub original_path: Option<String>,
    pub store_path: Option<String>,
    pub backup: Option<BackupConfiguration>,
    pub sync: Option<Vec<SyncConfiguration>>
}

#[derive(Deserialize)]
pub struct BackupConfiguration {
    #[serde(default="default_bool_false")]
    pub disabled: bool,
    pub backup_type: String,
    pub config: Value,
    pub check: Option<CheckConfiguration>,
    pub timeframes: Vec<Timeframe>
}

#[derive(Deserialize)]
pub struct SyncConfiguration {
    #[serde(default="default_bool_false")]
    pub disabled: bool,
    pub sync_type: String,
    pub interval: String,
    pub config: Value,
    pub check: Option<CheckConfiguration>,
    pub controller: Option<ControllerConfiguration>
}

#[derive(Deserialize)]
pub struct ControllerConfiguration {
    pub controller_type: String
}

#[derive(Deserialize)]
pub struct CheckConfiguration {
    pub check_type: String
}

#[derive(Deserialize)]
pub struct Timeframe {
    pub frame: String,
    #[serde(default="default_u32_1")]
    pub amount: u32
}

fn default_u32_1() -> u32 { 1 }
fn default_bool_false() -> bool { false }

#[derive(Deserialize)]
pub struct PathBase {
    #[serde(default="default_config_dir")]
    pub config_dir: String, // Here should be the configuration files
    #[serde(default="default_tmp_dir")]
    pub save_dir: String, // Default base directory for saves
    pub timeframes_file: Option<String>, // File containing timeframe definitions
    #[serde(default="default_tmp_dir")]
    pub tmp_dir: String, // Directory for temporary files
    pub auth_data_file: Option<String>, // File containing shared authentication information
}

fn default_config_dir() -> String { String::from("/etc/vbackup") }
fn default_save_dir() -> String { String::from("/var/vbackup") }
fn default_tmp_dir() -> String { String::from("/tmp/vbackup ")}

pub struct Paths {
    pub config_dir: String, // Here should be the configuration files
    pub save_dir: String, // Default base directory for saves
    pub timeframes_file: String, // File containing timeframe definitions
    pub tmp_dir: String, // Directory for temporary files
    pub auth_data_file: String // File containing shared authentication information
}

pub struct ModulePaths<'a> {
    pub base_paths: &'a Paths,
    pub original_path: Option<String>, // Path of the original directory to back up
    pub store_path: String, // Path for a local backup (or just path that will be synced)
    pub module_data_dir: String // Path for the modules to store additional data
}

impl Paths {
    pub fn from(base: PathBase) -> Paths {
        return Paths {
            timeframes_file: base.timeframes_file.unwrap_or(format!("{}/timeframes.json", &base.config_dir)),
            auth_data_file: base.auth_data_file.unwrap_or(format!("{}/auth_data.json", &base.config_dir)),
            config_dir: base.config_dir,
            save_dir: base.save_dir,
            tmp_dir: base.tmp_dir,
        }
    }

    pub fn for_module(&self, name: &str, module_type: &str, original_path: &Option<String>, save_path_option: &Option<String>) -> ModulePaths {
        let backup_path = if save_path_option.is_some() {
            String::from(save_path_option.as_ref().unwrap())
        } else {
            if original_path.is_none() {
                warn!("Using default store path (to sync from) when no backup was configured");
            }
            format!("{}/{}", self.save_dir.as_str(), name)
        };

        let module_data_dir = format!("{}/.module_data/{}/{}",
                                      self.save_dir.as_str(),
                                      module_type,
                                      name);

        return ModulePaths {
            base_paths: self,
            original_path: original_path.clone(),
            store_path: backup_path,
            module_data_dir
        }
    }
}