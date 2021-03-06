use crate::modules::traits::Sync;
use crate::util::objects::paths::{ModulePaths};
use crate::Arguments;

use serde_json::Value;

mod duplicati;
mod rsync;

pub struct SyncModule {
    module: Box<dyn SyncRelay>
}

impl SyncModule {
    pub fn new(sync_type: &str, name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Self,String> {
        let module: Box<dyn SyncRelay> = match sync_type.to_lowercase().as_str() {
            duplicati::Duplicati::MODULE_NAME => duplicati::Duplicati::new(name, config_json, paths, args)?,
            rsync::Rsync::MODULE_NAME => rsync::Rsync::new(name, config_json, paths, args)?,
            unknown => {
                let msg = format!("Unknown sync module: '{}'", unknown);
                error!("{}", msg);
                return Err(msg)
            }
        };

        return Ok(SyncModule { module } );
    }
}

impl SyncRelay for SyncModule {
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

pub trait SyncRelay {
    fn init(&mut self) -> Result<(), String>;
    fn sync(&self) -> Result<(), String>;
    fn restore(&self) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
    fn get_module_name(&self) -> &str;
}

impl<T: Sync> SyncRelay for T {
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