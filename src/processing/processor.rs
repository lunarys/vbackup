use crate::modules::object::{Configuration, TimeFrames, Paths, Arguments};
use crate::modules::reporting::ReportingModule;

pub fn process_configurations(args: &Arguments,
                              paths: &Paths,
                              timeframes: &TimeFrames,
                              reporter: &ReportingModule,
                              configurations: Vec<Configuration>) -> Result<(),String> {

}