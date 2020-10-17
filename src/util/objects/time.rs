use crate::util::io::{json,file};

use serde::{Deserialize,Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use chrono::{Local, DateTime};
use std::path::Path;

pub type TimeFrames = HashMap<String, TimeFrame>;

fn default_usize_1() -> usize { 1 }

// a collection of values for a single execution
pub struct ExecutionTiming {
    pub time_frame_reference: TimeFrameReference,
    pub time_frame: Rc<TimeFrame>,
    pub last_run: Option<TimeEntry>,
    pub execution_time: DateTime<Local>
}

// a reference to a timeframe definition
#[derive(Deserialize,Clone)]
pub struct TimeFrameReference {
    pub frame: String,
    #[serde(default="default_usize_1")]
    pub amount: usize
}

// a timeframe definition
#[derive(Clone,Deserialize)]
pub struct TimeFrame {
    pub identifier: String,
    pub interval: i64,
}

// savedata representation
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

// saves a timestamp with readable date
#[derive(Clone,Deserialize,Serialize)]
pub struct TimeEntry {
    // TODO: Maybe also add key here with flatten thingy or so
    pub timestamp: i64,
    pub date: Option<String> // TODO: Is there a better data type?
}