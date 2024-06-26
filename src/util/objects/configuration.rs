use crate::util::objects::time::TimeFrameReference;
use crate::util::objects::paths::SourcePath;

use serde_json::Value;
use serde::{Deserialize};

fn default_bool_false() -> bool { false }

#[derive(Deserialize,Clone)]
pub struct StrategyConfiguration {
    pub containers: Option<Vec<String>>,
    pub before: Option<Vec<String>>,
    pub after: Option<Vec<String>>
}

#[derive(Deserialize,Clone)]
pub struct Configuration {
    #[serde(default="default_bool_false")]
    pub disabled: bool,
    #[serde(default="default_bool_false")]
    pub manual: bool,
    pub name: String,
    pub savedata_in_store: Option<bool>,
    pub source_path: SourcePath,
    pub backup_path: Option<String>,
    pub backup: Option<BackupConfiguration>,
    pub sync: Option<SyncConfiguration>,
    pub setup: Option<StrategyConfiguration>
}

#[derive(Deserialize,Clone)]
pub struct BackupConfiguration {
    #[serde(default="default_bool_false")]
    pub disabled: bool,
    #[serde(rename(deserialize = "type"))]
    pub backup_type: String,
    pub config: Value,
    pub check: Option<Value>,
    pub timeframes: Vec<TimeFrameReference>,
    pub setup: Option<StrategyConfiguration>
}

#[derive(Deserialize,Clone)]
pub struct SyncConfiguration {
    #[serde(default="default_bool_false")]
    pub disabled: bool,
    #[serde(rename(deserialize = "type"))]
    pub sync_type: String,
    pub interval: TimeFrameReference,
    pub config: Value,
    pub check: Option<Value>,
    pub controller: Option<Value>,
    pub setup: Option<StrategyConfiguration>
}