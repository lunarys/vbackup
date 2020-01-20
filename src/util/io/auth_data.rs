// Local modules
use crate::util::io::json;
use crate::modules::object::Paths;

use crate::try_option;

// Other modules
use serde_json::Value;
use std::path::Path;
use serde::Deserialize;

fn load_from_file(name: &String, paths: &Paths) -> Result<Value,String> {
    let auth_file_content = json::from_file::<Value>(Path::new(&paths.auth_data_file))?;
    match auth_file_content.get(name) {
        Some(value) => Ok(value.clone()), // TODO: - clone
        None => Err("Key does not exist in file".to_string())
    }
}

fn load_if_reference(reference: &Option<String>, paths: &Paths) -> Result<Option<Value>, String> {
    if reference.is_some() {
        let value = load_from_file(reference.as_ref().unwrap(), paths)?;
        return Ok(Some(value));
    } else {
        return Ok(None);
    }
}

pub fn resolve<T>(reference: &Option<String>, config: &Option<Value>, paths: &Paths) -> Result<T,String> where for<'de> T: Deserialize<'de> {
    let value = match load_if_reference(reference, paths)? {
        Some(value) => value,
        None => try_option!(config.clone(), "Expected provided authentication, got none")
    };

    return json::from_value::<T>(value);
}