use std::collections::HashMap;
use crate::modules::traits::Sync;
use serde_json::Value;
use crate::modules::object::Paths;

use crate::{change_result};

pub mod duplicati;
pub mod rsync;

pub enum SyncModule {
    Duplicati(duplicati::Duplicati),
    Rsync(rsync::Rsync)
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

impl Sync for SyncModule {
    fn init(&mut self, name: &str, config_json: &Value, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String> {
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