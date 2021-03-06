use crate::{try_result};

use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};

use serde_json::Value;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub fn from_file<T>(file_name: &Path) -> Result<T, String> where for<'de> T: Deserialize<'de> {
    let file = try_result!(File::open(file_name), "Could not open file for reading");
    let buf_reader = BufReader::new(file);

    let result: Result<T,_> = serde_json::from_reader(buf_reader);
    return result.map_err(|err| format!("Failed reading the file '{}': {}", file_name.to_str().unwrap_or("?"), err));
}

pub fn from_file_checked<T>(file_name: &Path) -> Result<Option<T>, String> where for<'de> T: Deserialize<'de> {
    if file_name.exists() {
        return from_file::<T>(file_name).map(|r| Some(r));
    } else {
        return Ok(None);
    }
}

pub fn to_file<T: Serialize>(file_name: &Path, value: &T) -> Result<(), String> {
    let file_result = OpenOptions::new()
        .read(false)
        .truncate(true)
        .write(true)
        .create(true)
        .open(file_name);

    let file = try_result!(file_result, "Could not open file for writing");
    let writer = BufWriter::new(file);

    let result = serde_json::to_writer_pretty(writer, value);
    return result.map_err(|err| format!("Failed writing the file '{}': {}", file_name.to_str().unwrap_or("?"), err));
}

pub fn from_value<T>(value: Value) -> Result<T,String> where for<'de> T: Deserialize<'de> {
    let result: Result<T,_> = serde_json::from_value(value);
    return result.map_err(|err| format!("Could not parse object from json value ({})", err.to_string()));
}