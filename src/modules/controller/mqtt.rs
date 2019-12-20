use crate::modules::traits::Controller;
use crate::modules::object::Paths;

use crate::try_else;

use std::process::Command;
use serde_json::Value;
use serde::{Deserialize,Serialize};

pub struct MqttController {}

#[derive(Serialize,Deserialize)]
struct Configuration {
    start: bool,
    device: String,
    auth_reference: Option<String>,
    topic_sub: Option<String>,
    topic_pub: Option<String>,
    qos: Option<u8>
}

#[derive(Serialize,Deserialize)]
struct MqttConfiguration {
    host: String,
    port: Option<i32>,
    user: Option<String>,
    password: Option<String>,
    qos: Option<u8>
}

impl Controller for MqttController {
    fn begin(&self, name: String, config_json: Value, paths: Paths) -> Result<(), String> {
        debug!("MQTT controller start run is beginning");

        let config : Configuration = try_else!(serde_json::from_value(config_json), "Could not parse configuration");
        let mqtt_config : MqttConfiguration = match config.auth_reference {
            Some(value) => try_else!(serde_json::from_value(config_json), "Could not parse mqtt authentication"),
            None => try_else!(serde_json::from_value(config_json), "Could not parse mqtt configuration")
        };

        info!("Device name: {}", config.device);

        debug!("MQTT controller start run is done");
        return Ok(());
    }

    fn end(&self, name: String, config: Value, paths: Paths) -> Result<(), String> {
        unimplemented!()
    }
}