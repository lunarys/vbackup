use crate::modules::object::{TimeFrameReference, SaveData};
use crate::util::io::{file, json};

use crate::try_result;

use glob::{Paths};
use std::path::{PathBuf, Path};
use std::fs::remove_file;
use chrono::{DateTime, Local};
use std::collections::HashMap;

pub fn get_savedata(path: &str) -> Result<SaveData, String> {
    // Check if there is a file with savedata
    let savedata = if file::exists(path) {

        // File exists: Read savedata
        json::from_file::<SaveData>(Path::new(path))?
    } else {

        // File does not exist: Create new savedata
        SaveData {
            lastsave: HashMap::new(),
            nextsave: HashMap::new(),
            lastsync: HashMap::new()
        }
    };

    return Ok(savedata);
}

pub fn write_savedata(path: &str, savedata: &SaveData) -> Result<(), String> {
    json::to_file(Path::new(path), savedata)
}

pub fn time_format(date: &DateTime<Local>) -> String {
    return date.format("%Y-%m-%d %H:%M:%S").to_string();
}

pub fn format_filename(time: &DateTime<Local>, timeframe: &TimeFrameReference, name: &str, suffix_opt: Option<&str>, extension_opt: Option<&str>) -> String {
    // Output: Name for savefile
    // TODO: Could add replace CUSTOM from the timeframe with a custom name

    let iso_date = time.format("%Y-%m-%d_%H-%M-%S").to_string();

    let suffix = suffix_opt.unwrap_or("backup");

    if extension_opt.is_some() {
        return format!("{}_{}_{}_{}.{}", iso_date, timeframe.frame.as_str(), name, suffix, extension_opt.unwrap());
    } else {
        return format!("{}_{}_{}_{}", iso_date, timeframe.frame.as_str(), name, suffix);
    }
}

pub fn prune(directory: &str, identifier: &str, amount: &usize) -> Result<bool, String> {
    // Delete oldest savefiles if more than amount
    let pattern = format!("{}/*_{}_*", directory, identifier);
    let paths: Paths = try_result!(glob::glob(pattern.as_str()), "Could not read file list");

    let mut list: Vec<PathBuf> = paths.filter_map(Result::ok).collect();
    if list.len().gt(amount) {
        // Unstable sort works as file paths are unique and file names are prefixed with the ISO date
        list.sort_unstable();
        let oldest_file = list.first();
        if let Some(oldest_file_path) = oldest_file {
            debug!("Removing oldest file in timeframe: {:?}", oldest_file_path);
            if remove_file(oldest_file_path).is_err() {
                return Err(format!("Could not remove oldest file in '{}'", directory));
            }
        } else {
            // This should never happen... Backup is not executed if amount equals zero
            warn!("There is no oldest file to remove, the amount of saves seems to be zero, but still the backup was executed");
        }
        return Ok(true);
    } else {
        return Ok(false);
    };
}