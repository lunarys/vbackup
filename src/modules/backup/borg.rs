pub use crate::modules::shared::borg::Borg;
use crate::modules::traits::Backup;
use serde_json::Value;
use crate::util::objects::paths::ModulePaths;
use crate::Arguments;
use crate::util::objects::time::ExecutionTiming;

impl Backup for Borg {
    const MODULE_NAME: &'static str = "";

    fn new(name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Box<Self>, String> {
        Borg::new(name, config_json, paths, args)
    }

    fn init(&mut self) -> Result<(), String> {
        Borg::init(self)
    }

    fn backup(&self, time_frames: &Vec<ExecutionTiming>) -> Result<(), String> {
        todo!()
    }

    fn restore(&self) -> Result<(), String> {
        todo!()
    }

    fn clear(&mut self) -> Result<(), String> {
        Borg::clear(self)
    }
}