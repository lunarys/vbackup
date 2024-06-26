use std::rc::Rc;
use crate::modules::traits::Backup;
use crate::util::objects::time::{ExecutionTiming};
use crate::util::objects::paths::{ModulePaths};
use crate::Arguments;

use serde_json::Value;

mod tar7zip;
mod borg;

pub struct BackupModule {
    module: Box<dyn BackupWrapper>
}

impl BackupModule {
    pub fn new(backup_type: &str, name: &str, config_json: &Value, paths: ModulePaths, args: &Rc<Arguments>) -> Result<Self, String> {
        let module: Box<dyn BackupWrapper> = match backup_type.to_lowercase().as_str() {
            tar7zip::Tar7Zip::MODULE_NAME => {
                tar7zip::Tar7Zip::new(name, config_json, paths, args)?
            },
            <borg::Borg as Backup>::MODULE_NAME => {
                <borg::Borg as Backup>::new(name, config_json, paths, args)?
            },
            unknown => {
                let msg = format!("Unknown backup module: '{}'", unknown);
                error!("{}", msg);
                return Err(msg)
            }
        };

        return Ok(BackupModule { module });
    }
}

impl BackupWrapper for BackupModule {
    fn init(&mut self) -> Result<(), String> {
        self.module.init()
    }

    fn backup(&self, timings: &Vec<ExecutionTiming>) -> Result<(), String> {
        self.module.backup(timings)
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

pub trait BackupWrapper {
    fn init(&mut self) -> Result<(), String>;
    fn backup(&self, time_frames: &Vec<ExecutionTiming>) -> Result<(), String>;
    fn restore(&self) -> Result<(), String>;
    fn clear(&mut self) -> Result<(), String>;
    fn get_module_name(&self) -> &str;
}

impl<T: Backup> BackupWrapper for T {
    fn init(&mut self) -> Result<(), String> {
        Backup::init(self)
    }

    fn backup(&self, time_frames: &Vec<ExecutionTiming>) -> Result<(), String> {
        Backup::backup(self, time_frames)
    }

    fn restore(&self) -> Result<(), String> {
        Backup::restore(self)
    }

    fn clear(&mut self) -> Result<(), String> {
        Backup::clear(self)
    }

    fn get_module_name(&self) -> &str {
        Backup::get_module_name(self)
    }
}