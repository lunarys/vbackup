use crate::modules::traits::Controller;
use crate::modules::object::Paths;
use crate::util::auth_data;

use crate::try_result;
use crate::try_option;

use std::process::Command;
use serde_json::Value;
use serde::{Deserialize,Serialize};
use paho_mqtt as mqtt;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};
use paho_mqtt::Message;
use std::ops::Add;

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
    fn begin(&self, name: &String, config_json: &Value, paths: &Paths) -> Result<bool, String> {
        debug!("MQTT controller start run is beginning");

        let config : Configuration = try_result!(serde_json::from_value(config_json.clone()),
            "Could not parse configuration");

        info!("MQTT controller start run for device '{}' (start={})", config.device, config.start);

        let mqtt_config : MqttConfiguration = match config.auth_reference {
            Some(ref value) => {
                let auth_data = try_result!(auth_data::load(value, &paths),
                    "Could not get auth_data");
                try_result!(serde_json::from_value(auth_data.clone()),
                    "Could not parse mqtt authentication")
            },
            None => {
                try_result!(serde_json::from_value(config_json.clone()),
                    "Could not parse mqtt configuration")
            }
        };

        let qos = mqtt_config.qos.unwrap_or(1);
        let topic_pub = self.get_topic_pub(&config, &mqtt_config);
        let (client,receiver) : (mqtt::Client,Receiver<Option<mqtt::Message>>) =
            try_result!(self.get_client(&config, &mqtt_config), "Could not create mqtt client and receiver");

        trace!("Publish topic is '{}'", topic_pub);

        let result = if config.start {
            self.start_boot(&client, &receiver, topic_pub, qos)
        } else {
            self.start_run(&client, &receiver, topic_pub, qos)
        }?;

        debug!("MQTT controller start run is done");
        return Ok(result);
    }

    fn end(&self, name: &String, config_json: &Value, paths: &Paths) -> Result<bool, String> {
        debug!("MQTT controller end run is beginning");

        let config : Configuration = try_result!(serde_json::from_value(config_json.clone()),
            "Could not parse configuration");

        let mqtt_config : MqttConfiguration = match config.auth_reference {
            Some(ref value) => {
                let auth_data = try_result!(auth_data::load(value, &paths),
                    "Could not get auth_data");
                try_result!(serde_json::from_value(auth_data.clone()),
                    "Could not parse mqtt authentication")
            },
            None => {
                try_result!(serde_json::from_value(config_json.clone()),
                    "Could not parse mqtt configuration")
            }
        };

        let qos = mqtt_config.qos.unwrap_or(1);
        let topic_pub = self.get_topic_pub(&config, &mqtt_config);
        let (client,receiver) : (mqtt::Client,Receiver<Option<mqtt::Message>>) =
            try_result!(self.get_client(&config, &mqtt_config), "Could not create mqtt client and receiver");

        trace!("Publish topic is '{}'", topic_pub);

        let result = try_result!(self.end(client, topic_pub, qos), "Could not end in mqtt controller");

        debug!("MQTT controller end run is done");
        return Ok(result);
    }
}

impl MqttController {
    fn start_boot(&self, client: &mqtt::Client, receiver: &Receiver<Option<mqtt::Message>>, topic: String, qos: i32) -> Result<bool, String> {
        let msg = mqtt::Message::new(topic, "START_BOOT", qos);
        if client.publish(msg).is_err() {
            return Err("Could not send start initiation message".to_string());
        }

        // TODO: Timeout as option?
        let timeout = Duration::new(600, 0);
        let received: String = self.wait_for_message(receiver, timeout, None)?;

        if received.to_lowercase().eq("disabled") {
            info!("Device is disabled and thus not available");
        }

        // Return wether device is available or not
        Ok(received.to_lowercase().eq("ready"))
    }

    fn start_run(&self, client: &mqtt::Client, receiver: &Receiver<Option<mqtt::Message>>, topic: String, qos: i32) -> Result<bool, String> {
        let msg = mqtt::Message::new(topic, "START_RUN", qos);
        if client.publish(msg).is_err() {
            return Err("Could not send check initiation message".to_string());
        }

        // TODO: Timeout as option?
        let timeout = Duration::new(600, 0);
        let received: String = self.wait_for_message(receiver, timeout, None)?;

        if received.to_lowercase().eq("disabled") {
            info!("Device is disabled and thus not available");
        }

        // Return wether device is available or not
        Ok(received.to_lowercase().eq("ready"))
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

        let mqtt_host = format!("tcp://{}:{}", mqtt_config.host, mqtt_config.port.unwrap_or(1883));

        trace!("Trying to connect to mqtt broker with address '{}'", mqtt_host);

        let mut client: mqtt::Client = try_result!(mqtt::Client::new(mqtt_host), "Failed connecting to broker");

        let mut options_builder = mqtt::ConnectOptionsBuilder::new();
        let mut options = options_builder.clean_session(true);
        options = options.user_name(&mqtt_config.user);
        if mqtt_config.password.is_some() {
            options = options.password(mqtt_config.password.as_ref().unwrap());
        }

        //options.connect_timeout()
        //options.automatic_reconnect()
        //options.will_message()

        let topic_sub = self.get_topic_sub(&config, &mqtt_config);
        let qos = mqtt_config.qos.unwrap_or(1);

        trace!("Subscription topic is '{}'", topic_sub);

        let connection = try_result!(client.connect(options.finalize()),
                                     "Could not connect to the mqtt broker");

        let receiver = client.start_consuming();
        client.subscribe(&topic_sub, qos);

        Ok((client, receiver))
    }

    fn wait_for_message(&self, receiver: &Receiver<Option<mqtt::Message>>, timeout: Duration, expected: Option<String>) -> Result<String, String> {
        let start_time = Instant::now();

        loop {
            let time_left = timeout - start_time.elapsed();

            let received : Option<mqtt::Message> = try_result!(receiver.recv_timeout(timeout), "Timeout exceeded");
            // TODO: What was this again?
            let received_message: mqtt::Message = try_option!(received, "Timeout on receive operation");

            debug!("Received mqtt message '{}'", received_message.to_string());

            let received_string = received_message.payload_str().to_string();
            if expected.is_some() {
                if received_string.eq(expected.as_ref().unwrap()) {
                    return Ok(received_string);
                }
            } else {
                return Ok(received_string);
            }

            // Did not receive expected message -> Wait again
        }
    }

    fn get_topic_sub(&self, config: &Configuration, mqtt_config: &MqttConfiguration) -> String {
        config.topic_sub.clone().unwrap_or("device/".to_string()
            .add(&config.device)
            .add("/save/")
            .add(&mqtt_config.user)
            .add("/status"))
    }

    fn get_topic_pub(&self, config: &Configuration, mqtt_config: &MqttConfiguration) -> String {
        config.topic_pub.clone().unwrap_or("device/".to_string()
            .add(&config.device)
            .add("/save/")
            .add(&mqtt_config.user)
            .add("/status/desired"))
    }
}