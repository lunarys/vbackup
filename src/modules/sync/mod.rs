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
            "duplicati" => duplicati::Duplicati::new(name, config_json, paths, args)?,
            "rsync-ssh" | "rsync" => rsync::Rsync::new(name, config_json, paths, args)?,
            unknown => {
                let msg = format!("Unknown sync module: '{}'", unknown);
                error!("{}", msg);
                return Err(msg)
            }
        };

        return Ok(SyncModule { module } );
    }
}

impl Sync for SyncModule {
    fn new(_name: &str, _config_json: &Value, _paths: ModulePaths, _args: &Arguments) -> Result<Box<Self>, String> {
        return Err(String::from("Can not create anonymous sync module using the default trait method"));
    }

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
}

trait SyncRelay {
    fn init(&mut self) -> Result<(), String>;
    fn sync(&self) -> Result<(), String>;
    fn restore(&self) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
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
}