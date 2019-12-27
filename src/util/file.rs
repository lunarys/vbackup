use crate::modules::object::CommandWrapper;
use crate::{try_result, bool_result};

use std::process::{Command, Child, ExitStatus};
use std::io::{Write, BufReader, Read};
use std::fs::{OpenOptions, Permissions, File};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

pub fn write_with_perm(file_name: &str, mode: &str, to_write: &str, overwrite: bool) -> Result<(), String>{
    let file_result = OpenOptions::new()
        .read(false)
        .truncate(overwrite)
        .write(true)
        .create(true)
        .mode(try_result!(u32::from_str_radix(mode, 8), "Mode is not a number")) // Only sets mode when creating the file...
        .open(file_name);
    let mut file: File = try_result!(file_result, "Could not open file");

    set_permission(file_name, mode)?;

    try_result!(file.write_all(to_write.as_bytes()), "Could not write to file");
    try_result!(file.flush(), "Could not flush file");

    return Ok(());
}

pub fn write(file_name: &str, content: &str, overwrite: bool) -> Result<(), String> {
    let file_result = OpenOptions::new()
        .read(false)
        .truncate(overwrite)
        .write(true)
        .create(true)
        .open(file_name);
    let mut file: File = try_result!(file_result, "Could not open file for writing");

    try_result!(file.write_all(content.as_bytes()), "Could not write to file");
    try_result!(file.flush(), "Could not flush file");

    return Ok(());
}

pub fn read(file_name: &str) -> Result<String, String> {
    let file_result = OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .open(file_name);
    let mut file: File = try_result!(file_result, "Could not open file for reading");

    let mut file_content = String::new();
    try_result!(file.read_to_string(&mut file_content), "Could not read from file");

    return Ok(file_content);
}

pub fn write_if_change(file_name: &str, mode: Option<&str>, to_write: &str, overwrite: bool) -> Result<bool, String> {
    if exists(file_name) {
        let file_content = read(file_name)?;
        if file_content.eq(&to_write) {
            debug!("Not writing file, content is as desired");
            return Ok(false);
        }
    }

    if mode.is_some() {
        write_with_perm(file_name, mode.unwrap(), to_write, overwrite)?;
    } else {
        write(file_name, to_write, overwrite)?;
    }

    return Ok(true);
}

pub fn set_permission(file_name: &str, mode: &str) -> Result<(),String> {
    // Workaround for setting file access permissions
    let mut cmd = Command::new("chmod")
        .arg(mode)
        .arg(file_name)
        .spawn();

    let mut process: Child = try_result!(cmd, "Could not start process for setting file permissions");
    let result: ExitStatus = try_result!(process.wait(), "Process for setting file permissions failed");

    return bool_result!(result.success(), (), "Process for setting file permissions exited with error");
}

pub fn exists(file_name: &str) -> bool {
    Path::new(file_name).exists()
}