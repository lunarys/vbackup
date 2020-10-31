use crate::modules::check::Reference;
use crate::util::objects::configuration::Configuration;
use serde::{Deserialize};
use std::rc::Rc;

// fn default_bool_true() -> bool { true }
fn default_bool_false() -> bool { false }
fn default_config_dir() -> String { String::from("/etc/vbackup") }
fn default_save_dir() -> String { String::from("/var/vbackup") }
fn default_tmp_dir() -> String { String::from("/tmp/vbackup ")}

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
    #[serde(default="default_bool_false")]
    pub savedata_in_store: bool,
    pub reporting_file: Option<String>,
    pub docker_images: Option<String>,
    #[serde(default="default_bool_false")]
    pub no_docker: bool
}

#[derive(Clone)]
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

#[derive(Clone)]
pub struct ModulePaths {
    pub base_paths: Rc<Paths>,
    pub save_data: String, // Savedata file
    pub source: SourcePath, // Path of the original directory to back up
    pub destination: String, // Path for a local backup (or just path that will be synced)
    pub module_data_dir: String // Path for the modules to store additional data
}

#[derive(Deserialize,Clone)]
#[serde(untagged)]
pub enum SourcePath {
    Single(String),
    Multiple(Vec<PathMapping>)
}

#[derive(Deserialize,Clone)]
pub struct PathMapping {
    pub path: String,
    pub name: String
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
}

impl ModulePaths {
    pub fn for_check_module(paths: &Rc<Paths>, module_type: &str, config: &Configuration, reference: Reference) -> ModulePaths {
        return match reference {
            Reference::Backup => ModulePaths::for_backup_module(paths, module_type, config),
            Reference::Sync => ModulePaths::for_sync_module(paths, module_type, config)
        }
    }

    pub fn for_sync_module(paths: &Rc<Paths>, module_type: &str, config: &Configuration) -> ModulePaths {
        let name = config.name.as_str();
        let has_backup = config.backup.is_some();
        let backup_path = &config.backup_path;
        let source = &config.source_path;
        let source_alt = backup_path.clone().map(|opt| SourcePath::Single(opt));
        let source_opt = if has_backup {
            source_alt.as_ref()
        } else {
            Some(source)
        };
        let savedata_in_store = &config.savedata_in_store;
        let savedata_store = &source_opt;

        return ModulePaths::from_paths(paths, name, module_type, source_opt, None, savedata_in_store, savedata_store);
    }

    pub fn for_backup_module(paths: &Rc<Paths>, module_type: &str, config: &Configuration) -> ModulePaths {
        let name = config.name.as_str();
        let source = &config.source_path;
        let destination_opt = &config.backup_path;
        let savedata_in_store = &config.savedata_in_store;
        let savedata_store = &config.backup_path.clone().map(|path| SourcePath::Single(path));

        return ModulePaths::from_paths(paths, name, module_type, Some(source), destination_opt.as_ref(), savedata_in_store, &savedata_store.as_ref());
    }

    fn from_paths(from: &Rc<Paths>, name: &str, module_type: &str, source_opt: Option<&SourcePath>, destination_opt: Option<&String>, savedata_in_store: &Option<bool>, savedata_store: &Option<&SourcePath>) -> ModulePaths {
        let source = if source_opt.is_some() {
            source_opt.unwrap().clone()
        } else {
            SourcePath::Single(format!("{}/{}", from.save_dir.as_str(), name))
        };
        let destination = if destination_opt.is_some() {
            String::from(destination_opt.unwrap().as_str())
        } else {
            format!("{}/{}", from.save_dir.as_str(), name)
        };

        let module_data_base = format!("{}/.module_data/{}", from.save_dir.as_str(), name);
        let module_data_dir = format!("{}/{}", module_data_base.as_str(), module_type);

        let save_data = if savedata_in_store.unwrap_or(from.savedata_in_store) {
            if let Some(source_path) = savedata_store {
                if let SourcePath::Single(path) = source_path {
                    format!("{}/.savedata.json", path)
                } else {
                    // not supported, fall back to default location
                    warn!("Storing savedata with the backup is not possible with multiple paths... Using default location for module");
                    format!("{}/savedata.json", module_data_base.as_str())
                }
            } else {
                format!("{}/.savedata.json", &destination)
            }
        } else {
            format!("{}/savedata.json", module_data_base.as_str())
        };

        return ModulePaths {
            base_paths: from.clone(),
            save_data,
            source,
            destination,
            module_data_dir
        }
    }
}