use crate::rewrap;
use crate::try_result;

use std::fs::File;
use std::io::BufReader;
use std::io::prelude::Read;

use serde_json::Value;

pub fn from_file(file_name: &str) -> Result<Value, String> {
    let file = try_result!(File::open(file_name), "Could not open file");

    let mut buf_reader = BufReader::new(file);

    //let mut contents = String::new();
    //buf_reader.read_to_string(&mut contents)?;

    rewrap!(serde_json::from_reader(buf_reader), "Failed reading the file")
}

pub fn to_file(file_name: &str, value: &Value) -> Result<(), String> {
    Ok(())
}