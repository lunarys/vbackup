pub use crate::modules::shared::borg::Borg;
use crate::modules::traits::{Sync};
use serde_json::Value;
use crate::util::objects::paths::ModulePaths;
use crate::Arguments;

impl Sync for Borg {
    const MODULE_NAME: &'static str = "borg";

    fn new(name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Box<Self>, String> {
        Borg::new(name, config_json, paths, args)
    }

    fn init(&mut self) -> Result<(), String> {
        Borg::init(self)
    }

    fn sync(&self) -> Result<(), String> {
        todo!()
    }

    fn restore(&self) -> Result<(), String> {
        todo!()
    }

    fn clear(&mut self) -> Result<(), String> {
        Borg::clear(self)
    }
}