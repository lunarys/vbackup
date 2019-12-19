use std::collections::HashMap;
use crate::modules::traits::Sync;

pub mod duplicati;

pub fn get_module_list() -> HashMap<&'static str, impl Sync> {
    let mut modules = HashMap::new();

    modules.insert("duplicati", duplicati::Duplicati{});

    return modules;
}