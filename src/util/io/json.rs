use crate::{change_error,try_result};

use std::fs::File;
use std::io::{BufReader, BufWriter};

use serde_json::Value;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub fn from_file<T>(file_name: &Path) -> Result<T, String> where for<'de> T: Deserialize<'de> {
    let file = try_result!(File::open(file_name), "Could not open file for reading");
    let buf_reader = BufReader::new(file);

    let result: Result<T,_> = serde_json::from_reader(buf_reader);
    return result.map_err(|_| format!("Failed reading the file '{}'", "<filename here>"));
}

pub fn to_file<T: Serialize>(file_name: &Path, value: &T) -> Result<(), String> {
    let file = try_result!(File::open(file_name), "Could not open file for writing");
    let writer = BufWriter::new(file);

    let result = serde_json::to_writer_pretty(writer, value);
    return result.map_err(|_| format!("Failed writing the file '{}'", "<filename here>"));
}

pub fn from_value<T>(value: Value) -> Result<T,String> where for<'de> T: Deserialize<'de> {
    let result: Result<T,_> = serde_json::from_value(value);
    return result.map_err(|_| format!("Could not parse object from json value"));
}