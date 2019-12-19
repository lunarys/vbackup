use crate::modules::object::Arguments;

pub trait Backup {
    fn backup(&self, arguments: Arguments) -> Result<(), &str>;
    fn restore(&self, arguments: Arguments) -> Result<(), &str>;
}