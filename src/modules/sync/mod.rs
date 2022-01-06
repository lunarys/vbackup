use crate::modules::traits::Sync;
use crate::util::objects::paths::{ModulePaths};
use crate::Arguments;

use serde_json::Value;

mod duplicati;
mod rsync;
mod ssh_gpg;
mod borg;

pub struct SyncModule {
    module: Box<dyn SyncWrapper>
}

impl SyncModule {
    pub fn new(sync_type: &str, name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Self,String> {
        let module: Box<dyn SyncWrapper> = match sync_type.to_lowercase().as_str() {
            duplicati::Duplicati::MODULE_NAME => {
                duplicati::Duplicati::new(name, config_json, paths, args)?
            },
            rsync::Rsync::MODULE_NAME => {
                rsync::Rsync::new(name, config_json, paths, args)?
            },
            ssh_gpg::SshGpg::MODULE_NAME => {
                ssh_gpg::SshGpg::new(name, config_json, paths, args)?
            },
            borg::Borg::MODULE_NAME => {
                borg::Borg::new(name, config_json, paths, args)?
            },
            unknown => {
                let msg = format!("Unknown sync module: '{}'", unknown);
                error!("{}", msg);
                return Err(msg)
            }
        };

        return Ok(SyncModule { module } );
    }
}

impl SyncWrapper for SyncModule {
    fn init(&mut self) -> Result<(), String> {
        self.module.init()
    }

    fn sync(&self) -> Result<(), String> {
        self.module.sync()
    }

    fn restore(&self) -> Result<(), String> {
        self.module.restore()
    }

    fn clear(&mut self) -> Result<(), String> {
        self.module.clear()
    }

    fn get_module_name(&self) -> &str {
        self.module.get_module_name()
    }
}

pub trait SyncWrapper {
    fn init(&mut self) -> Result<(), String>;
    fn sync(&self) -> Result<(), String>;
    fn restore(&self) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
    fn get_module_name(&self) -> &str;
}

impl<T: Sync> SyncWrapper for T {
    fn init(&mut self) -> Result<(), String> {
        Sync::init(self)
    }

    fn sync(&self) -> Result<(), String> {
        Sync::sync(self)
    }

    fn restore(&self) -> Result<(), String> {
        Sync::restore(self)
    }

    fn clear(&mut self) -> Result<(), String> {
        Sync::clear(self)
    }

    fn get_module_name(&self) -> &str {
        Sync::get_module_name(self)
    }
}