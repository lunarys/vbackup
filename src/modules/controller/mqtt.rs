use crate::modules::traits::Controller;
use crate::modules::object::Paths;
use crate::util::auth_data;

use crate::{try_result,try_option,conf_resolve,auth_resolve};

use serde_json::Value;
use serde::{Deserialize};
use paho_mqtt as mqtt;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};
use std::ops::Add;

pub struct MqttController {}

#[derive(Deserialize)]
struct Configuration {
    start: bool,
    device: String,
    auth_reference: Option<String>,
    topic_sub: Option<String>,
    topic_pub: Option<String>,
    auth: Option<Value>
}

#[derive(Deserialize)]
struct MqttConfiguration {
    host: String,

    #[serde(default="default_port")]
    port: i32,

    user: String,
    password: Option<String>,

    #[serde(default="default_qos")]
    qos: i32
}

fn default_qos() -> i32 { 1 }
fn default_port() -> i32 { 1883 }

impl Controller for MqttController {
    fn begin(&self, name: &String, config_json: &Value, paths: &Paths) -> Result<bool, String> {
        debug!("MQTT controller start run is beginning");

        let config : Configuration = conf_resolve!(config_json);
        info!("MQTT controller start run for device '{}' (start={})", config.device, config.start);
        let mqtt_config: MqttConfiguration = auth_resolve!(&config.auth_reference, &config.auth, paths);

        let qos = mqtt_config.qos;
        let topic_pub = get_topic_pub(&config, &mqtt_config);
        let (client,receiver) : (mqtt::Client,Receiver<Option<mqtt::Message>>) =
            try_result!(get_client(&config, &mqtt_config), "Could not create mqtt client and receiver");

        trace!("Publish topic is '{}'", topic_pub);

        let result = start(&client, &receiver, config.start, topic_pub, qos)?;

        try_result!(client.disconnect(None), "Disconnect from broker failed");
        debug!("MQTT controller start run is done");
        return Ok(result);
    }

    fn end(&self, name: &String, config_json: &Value, paths: &Paths) -> Result<bool, String> {
        debug!("MQTT controller end run is beginning");

        let config : Configuration = conf_resolve!(config_json);
        info!("MQTT controller start run for device '{}' (start={})", config.device, config.start);
        let mqtt_config: MqttConfiguration = auth_resolve!(&config.auth_reference, &config.auth, paths);

        let qos = mqtt_config.qos;
        let topic_pub = get_topic_pub(&config, &mqtt_config);
        let (client,receiver) : (mqtt::Client,Receiver<Option<mqtt::Message>>) =
            try_result!(get_client(&config, &mqtt_config), "Could not create mqtt client and receiver");

        trace!("Publish topic is '{}'", topic_pub);

        let result = try_result!(end(&client, topic_pub, qos), "Could not end in mqtt controller");

        try_result!(client.disconnect(None), "Disconnect from broker failed");
        debug!("MQTT controller end run is done");
        return Ok(result);
    }
}

fn start(client: &mqtt::Client, receiver: &Receiver<Option<mqtt::Message>>, boot: bool, topic: String, qos: i32) -> Result<bool, String> {
    let msg = mqtt::Message::new(topic, if boot { "START_BOOT" } else { "START_RUN" }, qos);

    if client.publish(msg).is_err() {
        return Err("Could not send start initiation message".to_string());
    }

    // TODO: Timeout as option?
    let timeout = Duration::new(600, 0);
    let received: String = wait_for_message(receiver, timeout, None)?;

    if received.to_lowercase().eq("disabled") {
        info!("Device is disabled and thus not available");
    }

    // Return wether device is available or not
    Ok(received.to_lowercase().eq("ready"))
}

fn end(client: &mqtt::Client, topic: String, qos: i32) -> Result<bool, String> {
    let msg = mqtt::Message::new(topic, "DONE", qos);
    if client.publish(msg).is_ok() {
        Ok(true)
    } else {
        Err("Could not send end message".to_string())
    }
}

fn get_client(config: &Configuration, mqtt_config: &MqttConfiguration) -> Result<(mqtt::Client,Receiver<Option<mqtt::Message>>), String> {

    let mqtt_host = format!("tcp://{}:{}", mqtt_config.host, mqtt_config.port);

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

    let topic_sub = get_topic_sub(&config, &mqtt_config);
    let qos = mqtt_config.qos;

    trace!("Subscription topic is '{}'", topic_sub);

    try_result!(client.connect(options.finalize()), "Could not connect to the mqtt broker");

    let receiver = client.start_consuming();
    try_result!(client.subscribe(&topic_sub, qos), "Could not subscribe to mqtt topic");

    Ok((client, receiver))
}

fn wait_for_message(receiver: &Receiver<Option<mqtt::Message>>, timeout: Duration, expected: Option<String>) -> Result<String, String> {
    let start_time = Instant::now();

    loop {
        let time_left = timeout - start_time.elapsed();

        let received : Option<mqtt::Message> = try_result!(receiver.recv_timeout(time_left), "Timeout exceeded");
        // TODO: What was this again?
        let received_message: mqtt::Message = try_option!(received, "Timeout on receive operation");
        // TODO: Reconnect on connection loss

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

fn get_topic_sub(config: &Configuration, mqtt_config: &MqttConfiguration) -> String {
    config.topic_sub.clone().unwrap_or("device/".to_string()
        .add(&config.device)
        .add("/save/")
        .add(&mqtt_config.user)
        .add("/status"))
}

fn get_topic_pub(config: &Configuration, mqtt_config: &MqttConfiguration) -> String {
    config.topic_pub.clone().unwrap_or("device/".to_string()
        .add(&config.device)
        .add("/save/")
        .add(&mqtt_config.user)
        .add("/status/desired"))
}