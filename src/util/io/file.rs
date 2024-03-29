use crate::{try_result, bool_result};
use crate::util::command::CommandWrapper;
use crate::util::objects::paths::SourcePath;

use std::process::{Command, Child, ExitStatus};
use std::io::{Write, Read};
use std::fs;
use std::fs::{OpenOptions, File, read_dir, remove_file, rename};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

pub fn write_with_perm(file_name: &str, mode: &str, to_write: &str, overwrite: bool) -> Result<(), String>{
    let file_result = OpenOptions::new()
        .read(false)
        .truncate(overwrite)
        .write(true)
        .create(true)
        .mode(try_result!(u32::from_str_radix(mode, 8), "Mode is not a number")) // Only sets mode when creating the file...
        .open(file_name);
    let mut file: File = try_result!(file_result, format!("Could not open file '{}'", file_name));

    set_permission(file_name, mode)?;

    try_result!(file.write_all(to_write.as_bytes()), format!("Could not write to file '{}'", file_name));
    try_result!(file.flush(), format!("Could not flush file '{}'", file_name));

    return Ok(());
}

pub fn write(file_name: &str, content: &str, overwrite: bool) -> Result<(), String> {
    let file_result = OpenOptions::new()
        .read(false)
        .truncate(overwrite)
        .write(true)
        .create(true)
        .open(file_name);
    let mut file: File = try_result!(file_result, format!("Could not open file '{}' for writing", file_name));

    try_result!(file.write_all(content.as_bytes()), format!("Could not write to file '{}'", file_name));
    try_result!(file.flush(), format!("Could not flush file '{}'", file_name));

    return Ok(());
}

pub fn read(file_name: &str) -> Result<String, String> {
    let file_result = OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .open(file_name);
    let mut file: File = try_result!(file_result, format!("Could not open file '{}' for reading", file_name));

    let mut file_content = String::new();
    try_result!(file.read_to_string(&mut file_content), format!("Could not read from file '{}'", file_name));

    return Ok(file_content);
}

pub fn write_if_change(file_name: &str, mode: Option<&str>, to_write: &str, overwrite: bool) -> Result<bool, String> {
    if exists(file_name) {
        let file_content = read(file_name)?;
        if file_content.eq(&to_write) {
            debug!("Not writing file '{}', content is as desired", file_name);
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

pub fn move_file(from: &str, to: &str) -> Result<(),String> {
    if exists(from) {
        // TODO: Fails if from and to are on different filesystems...
        if let Err(err) = rename(from, to) {
            let err_new = format!("Could not move temporary backup to persistent file: {}", err.to_string());
            error!("{}", err_new);
            return Err(err_new);
        }

        return Ok(());
    } else {
        return Err(format!("File to move does not exist: {}", from));
    }
}

pub fn checked_remove(file_name: &str) -> Result<bool, String> {
    let path = Path::new(file_name);
    return if path.exists() {
        remove(file_name).map(|_| true)
    } else {
        Ok(false)
    }
}

pub fn remove(file_name: &str) -> Result<(), String> {
    let path = Path::new(file_name);
    return if let Err(err) = remove_file(path) {
        Err(format!("Could not remove file '{}' ({})", file_name, err.to_string()))
    } else {
        Ok(())
    }
}

pub fn set_permission(file_name: &str, mode: &str) -> Result<(),String> {
    // Workaround for setting file access permissions
    let cmd = Command::new("chmod")
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

pub fn create_dir_if_missing(dir_name: &str, also_parent: bool) -> Result<bool,String> {
    let path = Path::new(dir_name);
    return create_path_dir_if_missing(path, also_parent);
}

pub fn create_path_dir_if_missing(path: &Path, also_parent: bool) -> Result<bool,String> {
    return if path.exists() {
        if path.is_dir() {
            Ok(false)
        } else {
            Err(format!("Could not create directory '{}': A file with this name already exists", path.to_string_lossy()))
        }
    } else {
        let result = if also_parent {
            trace!("Creating missing directory '{}' (and possibly parent directories)", path.to_string_lossy());
            fs::create_dir_all(path)
        } else {
            trace!("Creating missing directory '{}'", path.to_string_lossy());
            fs::create_dir(path)
        };
        if let Err(err) = result {
            Err(format!("Could not create directory '{}': {}", path.to_string_lossy(), err))
        } else {
            Ok(true)
        }
    }
}

pub fn list_in_dir(dir_name: &str) -> Result<Vec<PathBuf>, String> {
    let path = Path::new(dir_name);

    if !path.is_dir() {
        return Err(String::from("Path is not a directory")); // TODO: throw
    }

    let result = try_result!(read_dir(path), "Could not read directory");
    // TODO: Not ideal, as any errors are silently ignored
    let files = result.filter_map(|r| {
        if r.is_ok() {
            Some(r.unwrap().path())
        } else {
            None
        }
    }).collect();

    return Ok(files);
}

pub fn _size(path: &SourcePath, no_docker: bool) -> Result<u64,String> {
    // TODO: Not ideal as it relies on other tools
    let mut cmd = if no_docker {
        let mut tmp = CommandWrapper::new("sh");
        tmp.arg_str("-c");
        tmp
    } else {
        let mut tmp = CommandWrapper::new("docker");
        tmp.arg_str("run")
            .arg_str("--rm")
            .arg_str("--name=vbackup-size-calc-tmp")
            .add_docker_volume_mapping(path, "volume")
            .arg_str("alpine")
            .arg_str("sh")
            .arg_str("-c");
        tmp
    };

    let cmd_path = if no_docker {
        if let SourcePath::Single(path) = path {
            path.as_str()
        } else {
            return Err(String::from("Multiple source paths are not supported in file size checks without docker"));
        }
    } else {
        "/volume"
    };

    cmd.arg_string(format!("du {} | tail -1 | cut -f1", cmd_path));

    let output = cmd.run_get_output()?;
    return output.parse().map_err(|_| String::from("Could not parse size in bytes from command output"));
}