pub use crate::modules::shared::borg::Borg;
use crate::modules::traits::{Sync};
use serde_json::Value;
use crate::util::objects::paths::ModulePaths;
use crate::Arguments;
use crate::modules::shared::ssh::SshConfig;
use crate::util::io::{auth_data, json};
use serde::{Deserialize};

#[derive(Deserialize)]
struct DeserializeBorgSyncConfig {
    host: Option<Value>,
    host_reference: Option<String>,

    #[serde(alias = "remote_directory")]
    directory: String
}

pub struct BorgSyncConfig {
    pub ssh_config: SshConfig,
    pub directory: String
}

impl Sync for Borg {
    const MODULE_NAME: &'static str = "borg";

    fn new(name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Box<Self>, String> {
        let config = json::from_value::<DeserializeBorgSyncConfig>(config_json.clone())?;
        let ssh_config = auth_data::resolve::<SshConfig>(&config.host_reference, &config.host, paths.base_paths.as_ref())?;

        Borg::new(name, config_json, paths, args, Some(BorgSyncConfig{
            ssh_config,
            directory: config.directory
        }))
    }

    fn init(&mut self) -> Result<(), String> {
        Borg::init(self)
    }

    fn sync(&self) -> Result<(), String> {
        Borg::run_save(self)
    }

    fn restore(&self) -> Result<(), String> {
        Borg::run_restore(self)
    }

    fn clear(&mut self) -> Result<(), String> {
        Borg::clear(self)
    }
}