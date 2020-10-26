use crate::modules::traits::Reporting;
use crate::util::objects::paths::{Paths};
use crate::util::objects::reporting::*;
use crate::Arguments;
use crate::{log_error};

use serde_json::Value;

mod mqtt;

pub struct ReportingModule {
    modules: Vec<Box<dyn ReportingRelay>>
}

use std::ops::Add;
use std::rc::Rc;

impl ReportingModule {
    pub fn new_combined(config_json: &Value, paths: &Rc<Paths>, args: &Arguments) -> Result<ReportingModule,String> {
        let array = if let Some(array) = config_json.as_array() {
            let mut vec = vec![];
            for value in array {
                vec.push(value);
            }
            vec
        } else {
            vec![config_json]
        };

        let mut modules: Vec<Result<Box<dyn ReportingRelay>, String>> = vec![];

        for (index,value) in array.iter().enumerate() {
            let mut parser_error = false;
            if let Some(reporter) = value.get("type") {
                if reporter.is_string() {
                    if let Some(name) = reporter.as_str() {
                        match name.to_lowercase().as_str() {
                            mqtt::Reporter::MODULE_NAME => {
                                modules.push(mqtt::Reporter::new(value, paths, args)
                                    .map(|boxed| boxed as Box<dyn ReportingRelay>)
                                    .map_err(|err| format!("Error in reporter {}: {}", index, err)));
                            },
                            unknown => {
                                let msg = format!("Unknown controller module at position {}: '{}'... Skipping this one", index, unknown);
                                error!("{}", msg);
                                return Err(msg);
                            }
                        }
                    } else {
                        parser_error = true;
                    }
                } else {
                    parser_error = true;
                }
            } else {
                let msg = format!("Reporting module at position {} specified without type... Skipping this one", index);
                error!("{}", msg);
                return Err(msg);
            }

            if parser_error {
                let msg = format!("Could not parse reporter config at position {}... Skipping this one", index);
                error!("{}", msg);
                return Err(msg);
            }
        }

        return Ok(ReportingModule { modules: accumulate(modules)? });
    }

    pub fn new_empty() -> ReportingModule {
        return ReportingModule { modules: vec![] };
    }

    pub fn report_status(&mut self, run_type: RunType, name: Option<String>, status: Status) {
        let result = self.report(ReportEvent::Status(StatusReport {
            module: name.map(|input| String::from(input)),
            status,
            run_type
        }));

        log_error!(result);
    }

    pub fn report_size(&mut self, run_type: RunType, size_type: SizeType, name: Option<String>, size: u64) {
        let result = self.report(ReportEvent::Size(SizeReport {
            module: name.map(|input| String::from(input)),
            size,
            run_type,
            size_type
        }));

        log_error!(result);
    }

    pub fn report_operation(&mut self, operation: OperationStatus) {
        let result = self.report(ReportEvent::Operation(operation));
        log_error!(result);
    }
}

impl ReportingRelay for ReportingModule {
    fn init(&mut self) -> Result<(), String> {
        let result = self.modules.iter_mut().map(|module| {
            module.init()
        }).collect();

        return accumulate(result).map(|_| ());
    }

    fn report(&mut self, event: ReportEvent) -> Result<(), String> {
        let result = self.modules.iter_mut().map(|module| {
            module.report(event.clone())
        }).collect();

        return accumulate(result).map(|_| ());
    }

    fn clear(&mut self) -> Result<(), String> {
        let result = self.modules.iter_mut().map(|module| {
            module.clear()
        }).collect();

        return accumulate(result).map(|_| ());
    }
}

fn accumulate<T>(input: Vec<Result<T,String>>) -> Result<Vec<T>,String> {
    if input.iter().any(|r| r.is_err()) {
        let acc = input.iter()
            .filter_map(|r| r.as_ref().err())
            .fold(String::new(), |s,t| {
                let tmp = if String::new().ne(&s) {
                    s.add(", ")
                } else {
                    s
                };

                tmp.add(t.as_str())
            });
        return Err(acc);
    } else {
        let mut result_vec = vec![];
        for result in input {
            if result.is_ok() {
                result_vec.push(result.unwrap())
            }
        }
        return Ok(result_vec);
    }
}

pub trait ReportingRelay {
    fn init(&mut self) -> Result<(),String>;
    fn report(&mut self, event: ReportEvent) -> Result<(),String>;
    fn clear(&mut self) -> Result<(), String>;
}

impl<T: Reporting> ReportingRelay for T {
    fn init(&mut self) -> Result<(), String> {
        Reporting::init(self)
    }

    fn report(&mut self, event: ReportEvent) -> Result<(), String> {
        Reporting::report(self, event)
    }

    fn clear(&mut self) -> Result<(), String> {
        Reporting::clear(self)
    }
}