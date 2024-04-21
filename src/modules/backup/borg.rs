use std::rc::Rc;
pub use crate::modules::shared::borg::Borg;
use crate::modules::traits::Backup;
use serde_json::Value;
use crate::util::objects::paths::ModulePaths;
use crate::Arguments;
use crate::util::objects::time::ExecutionTiming;

impl Backup for Borg {
    const MODULE_NAME: &'static str = "borg";

    fn new(name: &str, config_json: &Value, paths: ModulePaths, args: &Rc<Arguments>) -> Result<Box<Self>, String> {
        Borg::new(name, config_json, paths, args, None)
    }

    fn init(&mut self) -> Result<(), String> {
        Borg::init(self)
    }

    fn backup(&self, _time_frames: &Vec<ExecutionTiming>) -> Result<(), String> {

        // ignore timeframes for now and just create a backup if necessary in any timeframe
        Borg::run_save(self)
    }

    fn restore(&self) -> Result<(), String> {
        Borg::run_restore(self)
    }

    fn clear(&mut self) -> Result<(), String> {
        Borg::clear(self)
    }
}