// Local modules
use crate::util::json;
use crate::util::auth_data;
use crate::modules::object::Paths;

// Local macros
use crate::try_else;

// Other modules
use serde_json::Value;

pub fn load(name: &String, paths: &Paths) -> Result<Value,String> {
    let auth_file_content = json::from_file(&paths.auth_data_file)?;
    match auth_file_content.get(name) {
        Some(value) => Ok(value.clone()),
        None => Err("Key does not exist in file".to_string())
    }
}