use crate::{change_error,try_result};

use std::fs::File;
use std::io::BufReader;

use serde_json::Value;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub fn from_file<T>(file_name: &Path) -> Result<T, String> where for<'de> T: Deserialize<'de> {
    let file = try_result!(File::open(file_name), "Could not open file");
    let buf_reader = BufReader::new(file);

    let result: Result<T,String> = change_error!(serde_json::from_reader(buf_reader), "Failed reading the file");
    return result;
}

pub fn to_file<T: Serialize>(file_name: &str, value: &T) -> Result<(), String> {
    Ok(())
}

pub fn from_value<T>(value: Value) -> Result<T,String> where for<'de> T: Deserialize<'de> {
    let result: Result<T,String> = change_error!(serde_json::from_value(value), "Could not parse object from json value");
    return result;
}