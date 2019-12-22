// Local modules
use crate::util::json;
use crate::modules::object::Paths;

// Other modules
use serde_json::Value;

pub fn load_from_file(name: &String, paths: &Paths) -> Result<Value,String> {
    let auth_file_content = json::from_file(&paths.auth_data_file)?;
    match auth_file_content.get(name) {
        Some(value) => Ok(value.clone()),
        None => Err("Key does not exist in file".to_string())
    }
}

pub fn load_if_reference(reference: &Option<String>, paths: &Paths) -> Result<Option<Value>, String> {
    if reference.is_some() {
        match load_from_file(reference.as_ref().unwrap(), paths) {
            Ok(value) => Ok(Some(value)),
            Err(err) => Err(err)
        }
    } else {
        Ok(None)
    }
}