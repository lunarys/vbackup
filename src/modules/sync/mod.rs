use crate::modules::traits::Sync;
use crate::modules::object::{Paths, ModulePaths};

use crate::{change_result};

use serde_json::Value;

mod duplicati;
mod rsync;

pub enum SyncModule<'a> {
    Duplicati(duplicati::Duplicati<'a>),
    Rsync(rsync::Rsync<'a>)
}

use SyncModule::*;

pub fn get_module(name: &str) -> Result<SyncModule,String> {
    return Ok(match name.to_lowercase().as_str() {
        "duplicati" => Duplicati(duplicati::Duplicati::new_empty()),
        "rsync" => Rsync(rsync::Rsync::new_empty()),
        unknown => {
            let msg = format!("Unknown sync module: '{}'", unknown);
            error!("{}", msg);
            return Err(msg)
        }
    })
}

impl<'a> Sync<'a> for SyncModule<'a> {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool) -> Result<(), String> {
        return match self {
            Duplicati(sync) => sync.init(name, config_json, paths, dry_run, no_docker),
            Rsync(sync) => sync.init(name, config_json, paths, dry_run, no_docker)
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