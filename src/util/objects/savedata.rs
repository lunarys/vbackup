use std::collections::HashMap;
use std::path::Path;
use serde::{Deserialize,Serialize};

use crate::util::objects::time::TimeEntry;
use crate::util::io::{file, json};

#[derive(Clone,Deserialize)]
pub struct SaveDataDeserialized {
    pub lastsave: HashMap<String,TimeEntry>,
    pub nextsave: HashMap<String,TimeEntry>,
    pub lastsync: HashMap<String,TimeEntry>
}

impl SaveDataDeserialized {
    pub fn into_serializable(self, path: &str) -> SaveData {
        return SaveData::from_deserialized(self, path);
    }
}

#[derive(Clone,Serialize)]
pub struct SaveData {
    pub lastsave: HashMap<String,TimeEntry>,
    pub nextsave: HashMap<String,TimeEntry>,
    pub lastsync: HashMap<String,TimeEntry>,
    #[serde(skip)]
    pub path: String
}

impl SaveData {
    pub fn from_deserialized(deserialized: SaveDataDeserialized, path: &str) -> Self {
        return Self {
            lastsave: deserialized.lastsave,
            nextsave: deserialized.nextsave,
            lastsync: deserialized.lastsync,
            path: String::from(path)
        }
    }

    pub fn create_directory_if_missing(&self) -> Result<bool, String> {
        let parent_dir_option = Path::new(self.path.as_str()).parent();
        return if let Some(parent_dir) = parent_dir_option {
            file::create_path_dir_if_missing(parent_dir, false)
        } else {
            warn!("Path for the savedata file is in the filesystem root, won't be creating a designated directory");

            // Suppose this behavior is intended (?)
            Ok(false)
        }
    }

    pub fn write(&self) -> Result<(),String> {
        trace!("Writing new savedata to '{}'", self.path.as_str());
        json::to_file(Path::new(&self.path), self)
    }
}

// Store all the savedata in a single hash map, in order to have it mutable everywhere
pub type SaveDataCollection = HashMap<String, SaveData>;

