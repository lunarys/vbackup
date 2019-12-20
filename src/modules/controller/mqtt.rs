use crate::modules::traits::Controller;
use crate::modules::object::Paths;
use crate::util::auth_data;

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
    fn begin(&self, name: String, config_json: &Value, paths: &Paths) -> Result<(), String> {
        debug!("MQTT controller start run is beginning");

        let config : Configuration = try_else!(serde_json::from_value(config_json.clone()),
            "Could not parse configuration");

        let mqtt_config : MqttConfiguration = match config.auth_reference {
            Some(value) => {
                let auth_data = try_else!(auth_data::load(&value, &paths),
                    "Could not get auth_data");
                try_else!(serde_json::from_value(auth_data.clone()),
                    "Could not parse mqtt authentication")
            },
            None => {
                try_else!(serde_json::from_value(config_json.clone()),
                    "Could not parse mqtt configuration")
            }
        };

        info!("Device name: {}", config.device);

        debug!("MQTT controller start run is done");
        return Ok(());
    }

    fn end(&self, name: String, config: &Value, paths: &Paths) -> Result<(), String> {
        debug!("MQTT controller end run is beginning");

        let config : Configuration = try_else!(serde_json::from_value(config_json.clone()),
            "Could not parse configuration");

        let mqtt_config : MqttConfiguration = match config.auth_reference {
            Some(value) => {
                let auth_data = try_else!(auth_data::load(&value, &paths),
                    "Could not get auth_data");
                try_else!(serde_json::from_value(auth_data.clone()),
                    "Could not parse mqtt authentication")
            },
            None => {
                try_else!(serde_json::from_value(config_json.clone()),
                    "Could not parse mqtt configuration")
            }
        };

        debug!("MQTT controller end run is done");
        return Ok(());
    }
}

impl MqttController {
    fn start(config: Configuration, mqtt_config: MqttConfiguration) -> Result<> {
        // TODO: Maybe just pass mqtt client object
    }

    fn check(config: Configuration, mqtt_config: MqttConfiguration) -> Result<> {

    }

    fn end(config: Configuration, mqtt_config: MqttConfiguration) -> Result<> {

    }
}