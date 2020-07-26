use crate::modules::traits::Sync;
use crate::util::objects::paths::{ModulePaths};
use crate::Arguments;

use serde_json::Value;

mod duplicati;
mod rsync;

pub enum SyncModule {
    Duplicati(duplicati::Duplicati),
    Rsync(rsync::Rsync)
}

use SyncModule::*;

pub fn get_module(name: &str) -> Result<SyncModule,String> {
    return Ok(match name.to_lowercase().as_str() {
        "duplicati" => Duplicati(duplicati::Duplicati::new_empty()),
        "rsync-ssh" => Rsync(rsync::Rsync::new_empty()),
        unknown => {
            let msg = format!("Unknown sync module: '{}'", unknown);
            error!("{}", msg);
            return Err(msg)
        }
    })
}

impl Sync for SyncModule {
    fn init(&mut self, name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<(), String> {
        return match self {
            Duplicati(sync) => sync.init(name, config_json, paths, args),
            Rsync(sync) => sync.init(name, config_json, paths, args)
        };
    }

    fn sync(&self) -> Result<(), String> {
        return match self {
            Duplicati(sync) => sync.sync(),
            Rsync(sync) => sync.sync()
        }
    }

    fn restore(&self) -> Result<(), String> {
        return match self {
            Duplicati(sync) => sync.restore(),
            Rsync(sync) => sync.restore()
        }
    }

    fn clear(&mut self) -> Result<(), String> {
        return match self {
            Duplicati(sync) => sync.clear(),
            Rsync(sync) => sync.clear()
        }
    }
}