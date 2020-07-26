use crate::modules::traits::Reporting;
use crate::modules::object::{Paths,Arguments};

use crate::try_option;

use serde_json::Value;

mod mqtt;

pub enum ReportingModule {
    Combined(Reporter),
    Mqtt(mqtt::Reporter),
    Empty // ?
}

use ReportingModule::*;
use std::ops::Add;
use std::rc::Rc;

fn get_module(name: &str) -> Result<ReportingModule, String> {
    return Ok(match name.to_lowercase().as_str() {
        "mqtt" => Mqtt(mqtt::Reporter::new_empty()),
        unknown => {
            let msg = format!("Unknown controller module: '{}'", unknown);
            error!("{}", msg);
            return Err(msg)
        }
    })
}

impl ReportingModule {
    pub fn new_combined() -> ReportingModule {
        return ReportingModule::Combined(Reporter::new_empty());
    }

    pub fn new_empty() -> ReportingModule {
        return ReportingModule::Empty;
    }
}

impl Reporting for ReportingModule {
    fn init(&mut self, config_json: &Value, paths: &Rc<Paths>, args: &Arguments) -> Result<(), String> {
        return match self {
            Combined(reporter) => reporter.init(config_json, paths, args),
            Mqtt(reporter) => reporter.init(config_json, paths, args),
            Empty => Ok(())
        }
    }

    fn report(&self, context: Option<&[&str]>, value: &str) -> Result<(), String> {
        return match self {
            Combined(reporter) => reporter.report(context, value),
            Mqtt(reporter) => reporter.report(context, value),
            Empty => Ok(())
        }
    }

    fn clear(&mut self) -> Result<(), String> {
        return match self {
            Combined(reporter) => reporter.clear(),
            Mqtt(reporter) => reporter.clear(),
            Empty => Ok(())
        }
    }
}

pub struct Reporter {
    bind: Option<Bind>
}

struct Bind {
    modules: Vec<ReportingModule>
}

impl Reporter {
    pub fn new_empty() -> Reporter {
        return Reporter{ bind: None };
    }
}

impl Reporting for Reporter {
    fn init(&mut self, config_json: &Value, paths: &Rc<Paths>, args: &Arguments) -> Result<(), String> {
        if self.bind.is_some() {
            let msg = String::from("Reporting module is already bound");
            error!("{}", msg);
            return Err(msg);
        }

        let array = if let Some(array) = config_json.as_array() {
            let mut vec = vec![];
            for value in array {
                vec.push(value)
            }
            vec
        } else {
            vec![config_json]
        };

        let result = array.iter().map(|value| {
            if let Some(reporter) = value.get("type") {
                if reporter.is_string() {
                    let mut result = get_module(reporter.as_str().unwrap())?;
                    result.init(*value, paths, args)?;
                    return Ok(result);
                } else {
                    return Err(String::from(""));
                }
            } else {
                return Err(String::from(""));
            }
        }).collect();

        let modules = accumulate(result)?;

        self.bind = Some(Bind {
            modules
        });

        return Ok(());
    }

    fn report(&self, context: Option<&[&str]>, value: &str) -> Result<(), String> {
        let bound: &Bind = try_option!(self.bind.as_ref(), "Reporting module is not bound");

        let result = bound.modules.iter().map(|module| {
            module.report(context, value)
        }).collect();

        return accumulate(result).map(|_| ());
    }

    fn clear(&mut self) -> Result<(), String> {
        let mut bound: Bind = try_option!(self.bind.take(), "Reporting module is not bound");

        let result = bound.modules.iter_mut().map(|module| {
            module.clear()
        }).collect();

        self.bind = None;

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