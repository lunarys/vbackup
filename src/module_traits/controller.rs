use crate::modules::object::Arguments;

pub trait Controller {
    fn begin(&self, arguments: Arguments) -> Result<(), String>;
    fn end(&self, arguments: Arguments) -> Result<(), String>;
}