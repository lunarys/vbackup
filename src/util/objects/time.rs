use serde::{Deserialize,Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use chrono::{Local, DateTime};

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

// saves a timestamp with readable date
#[derive(Clone,Deserialize,Serialize)]
pub struct TimeEntry {
    // TODO: Maybe also add key here with flatten thingy or so
    pub timestamp: i64,
    pub date: Option<String> // TODO: Is there a better data type?
}