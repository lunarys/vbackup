use crate::modules::object::Arguments;

pub trait Sync {
    fn sync(&self, arguments: Arguments) -> Result<(), &str>;
    fn restore(&self, arguments: Arguments) -> Result<(), &str>;
}