use std::collections::HashMap;
use crate::modules::traits::Controller;

pub mod mqtt;

pub fn get_module_list() -> HashMap<&'static str, impl Controller> {
    let mut modules = HashMap::new();

    modules.insert("mqtt", mqtt::MqttController{});

    return modules;
}