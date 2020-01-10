use crate::modules::traits::Reporting;
use crate::modules::object::ModulePaths;

use serde_json::Value;

mod mqtt;

pub enum ReportingModule {
    Mqtt(mqtt::Mqtt),
    Combined(Vec<ReportingModule>),
    None // ?
}

use ReportingModule::*;
use std::ops::Add;
use serde_json::value::Index;

fn init_all(config_json: &Value, dry_run: bool, no_docker: bool) -> Result<ReportingModule, String> {
    let mut module = if let Some(array) = config_json.as_array() {
        let (oks, errs) : (Vec<Result<ReportingModule,String>>,Vec<Result<ReportingModule,String>>) = array.iter()
            .map(get_module_init)
            .partition(Result::is_ok);

        if errs.is_empty() {
            let mut modules: Vec<ReportingModule> = oks.iter().map(Result::unwrap).collect();
            Combined(modules)
        } else {
            let err_acc = errs.iter().fold(String::new(), |s,t| s.add(t.unwrap_err().as_str()));
            return Err(err_acc);
        }
    } else {
        return Err(String::from("Init all for reporting called with no array"))
    };

    return Ok(module);
}

fn get_module_init(config_json: &Value, dry_run: bool, no_docker: bool) -> Result<ReportingModule, String> {
    if config_json.is_array() {
        return init_all(config_json, dry_run, no_docker)
    } else if let Some(reporter) = config_json.get("type") {
        if reporter.is_string() {
            let result = match reporter.as_str().unwrap() {
                "mqtt" => mqtt::new_empty(),
                unknown => {
                    let msg = format!("Unknown sync module: '{}'", unknown);
                    error!("{}", msg);
                    return Err(msg)
                }
            };
            result.init(config_json, dry_run, no_docker);
            return Ok(result);
        } else {
            return Err(String::from(""));
        }
    } else {
        return Err(String::from(""));
    }
}

impl Reporting for ReportingModule {
    fn init(&mut self, config_json: &Value, dry_run: bool, no_docker: bool) -> Result<(), String> {
        return match self {
            Mqtt(reporter) => reporter.init(config_json, dry_run, no_docker),
            Combined(list) => list.iter().map(ReportingModule::init).fold(Ok(()), Result::and)
        }
    }

    fn report(&self, context: &Option<&Vec<&str>>, kind: &str, value: String) -> Result<(), String> {
        return match self {
            Mqtt(reporter) => reporter.report(context, kind, value),
            Combined(list) => list.iter().map(ReportingModule::report).fold(Ok(()), Result::and) // TODO: Accumulate errors?
        }
    }

    fn clear(&mut self) -> Result<(), String> {
        return match self {
            Mqtt(reporter) => reporter.clear(),
            Combined(list) => list.iter().map(ReportingModule::clear).fold(Ok(()), Result::and) // TODO: Accumulate errors?
        }
    }
}