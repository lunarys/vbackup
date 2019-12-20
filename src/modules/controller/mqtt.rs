use crate::modules::traits::Controller;
use crate::modules::object::Paths;
use crate::util::auth_data;

use crate::try_else;

use std::process::Command;
use serde_json::Value;
use serde::{Deserialize,Serialize};
use paho_mqtt as mqtt;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use paho_mqtt::Message;

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
    user: String,
    password: Option<String>,
    qos: Option<i32>
}

impl Controller for MqttController {
    fn begin(&self, name: String, config_json: &Value, paths: &Paths) -> Result<bool, String> {
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

        let qos = mqtt_config.qos.unwrap_or(1);
        let topic_pub = self.get_topic_pub(&config, &mqtt_config);
        let (client,receiver) : (mqtt::Client,Receiver<Option<mqtt::Message>>) =
            try_else!(self.get_client(&config, &mqtt_config), "Could not create mqtt client and receiver");



        debug!("MQTT controller start run is done");
        return Ok(());
    }

    fn end(&self, name: String, config: &Value, paths: &Paths) -> Result<bool, String> {
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

        let qos = mqtt_config.qos.unwrap_or(1);
        let topic_pub = self.get_topic_pub(&config, &mqtt_config);
        let (client,receiver) : (mqtt::Client,Receiver<Option<mqtt::Message>>) =
            try_else!(self.get_client(&config, &mqtt_config), "Could not create mqtt client and receiver");

        let result = try_else!(self.end(client, topic_pub, qos), "Could not end in mqtt controller");

        debug!("MQTT controller end run is done");
        return Ok(result);
    }
}

impl MqttController {
    fn start(&self, client: mqtt::Client, receiver: &Receiver<Option<mqtt::Message>>, topic: String, qos: i32) -> Result<bool, String> {

    }

    fn check(&self, client: mqtt::Client, receiver: &Receiver<Option<mqtt::Message>>, topic: String, qos: i32) -> Result<bool, String> {
        let msg = mqtt::Message::new(topic, "CHECK", qos);
        if client.publish(msg).is_err() {
            return Err("Could not send check initiation message".to_string());
        }

        let received: Option<Message> = try_else!(receiver.recv(), "Could not receive mqtt message");
        // TODO: Stopped here
    }

    fn end(&self, client: mqtt::Client, topic: String, qos: i32) -> Result<bool, String> {
        let msg = mqtt::Message::new(topic, "DONE", qos);
        if client.publish(msg).is_ok() {
            Ok(true)
        } else {
            Err("Could not send end message".to_string())
        }
    }

    fn get_client(&self, config: &Configuration, mqtt_config: &MqttConfiguration) -> Result<(mqtt::Client,Receiver<Option<mqtt::Message>>), String> {
        let mut client: mqtt::Client = !try_else!(mqtt::Client::new("tcp://"), "Failed connecting to broker");

        let mut options = mqtt::ConnectOptionsBuilder::new().clean_session(true);
        options = options.user_name(mqtt_config.user.unwrap());
        if mqtt_config.password.is_some() {
            options = options.password(mqtt_config.password.unwrap());
        }

        //options.connect_timeout()
        //options.automatic_reconnect()
        //options.will_message()

        let topic_sub = self.get_topic_sub(&config, &mqtt_config);
        let qos = mqtt_config.qos.unwrap_or(1);

        if client.connect(options.finalize()).is_ok() {
            let receiver = client.start_consuming();
            receiver.recv_timeout(Duration::from_secs(600)); // TODO: Timeout option?

            client.subscribe(&topic_sub, qos);

            Ok((client, receiver))
        } else {
            Err("Could not connect to the mqtt broker".to_string())
        }
    }

    fn get_topic_sub(&self, config: &Configuration, mqtt_config: &MqttConfiguration) -> String {
        config.topic_sub.unwrap_or("device/".to_string()
            .add(&config.device)
            .add("/save/")
            .add(&mqtt_config.user)
            .add("/status"))
    }

    fn get_topic_pub(&self, config: &Configuration, mqtt_config: &MqttConfiguration) -> String {
        config.topic_pub.unwrap_or("device/".to_string()
            .add(&config.device)
            .add("/save/")
            .add(&mqtt_config.user)
            .add("/status/desired"))
    }
}