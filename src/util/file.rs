use std::fs::{OpenOptions, Permissions, File};
use crate::modules::object::CommandWrapper;

use crate::{try_result};
use std::process::{Command, Child, ExitStatus};
use std::io::{Write, BufReader, Read};

pub fn write_file_with_perm(file_name: &str, mode: &str, content: &str, overwrite: bool) -> Result<(), String>{
    // TODO: File exists?
    // Read from file
    let file_content = read_file(file_name)?;

    // Only write if it will change the content of the file
    let to_write = content.to_string() + "\n";
    if file_content.eq(&to_write) {
        println!("No need to write, content of file is already as desired!");
        return Ok(());
    }

    // TODO: File should exist for that...
    // Only set permission if file is changed
    set_file_permission(file_name, mode)?;
    write_file(file_name, &to_write, overwrite)?;

    return Ok(());
}

pub fn write_file(file_name: &str, content: &str, overwrite: bool) -> Result<(), String> {
    let file_result = OpenOptions::new()
        .read(false)
        .write(true)
        .create(true)
        .open(file_name);
    let file: File = try_result!(file_result, "Could not open file for writing");

    // TODO: Overwrite?
    try_result!(file.write_all(content.as_bytes()), "Could not write to file");
    try_result!(file.flush(), "Could not flush file");

    return Ok(());
}

pub fn read_file(file_name: &str) -> Result<String, String> {
    let file_result = OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .open(file_name);
    let file: File = try_result!(file_result, "Could not open file for reading");

    let mut file_content = String::new();
    try_result!(file.read_to_string(&file_content), "Could not read from file");

    return Ok(file_content);
}

pub fn write_file_if_change(file_name: &str, mode: Option<&str>, content: &str, overwrite: bool) -> Result<bool, String> {
    let file_content = read_file_content(file_name)?;
    let to_write = String::from(content) + "\n";

    if file_content.eq(&to_write) {
        debug!("Not writing file, content is as desired");
        return Ok(false);
    }

    if mode.is_some() {
        write_file_with_perm(file_name, mode.unwrap(), &to_write, overwrite)?;
    } else {
        write_file(file_name, &to_write, overwrite)?;
    }

    return Ok(true);
}

pub fn set_file_permission(file_name: &str, mode: &str) -> Result<(),String> {
    // Workaround for setting file access permissions
    let mut cmd = Command::new("chmod")
        .arg(mode)
        .arg(file_name)
        .spawn();

    let process: Child = try_result!(cmd, "Could not start process for setting file permissions");
    let result: ExitStatus = try_result!(process.wait(), "Process for setting file permissions failed");

    return if result.success() {
        Ok(())
    } else {
        Err(String::from("Process for setting file permissions exited with error"))
    }
}