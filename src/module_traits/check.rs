use crate::modules::object::Arguments;

pub trait Check {
    fn check(&self, arguments: Arguments) -> Result<(), &str>;
    fn update(&self, arguments: Arguments) -> Result<(), &str>;
}