use crate::modules::traits::Controller;
use crate::modules::object::ModulePaths;
use crate::util::auth_data;

use crate::{try_result,try_option,bool_result,conf_resolve,auth_resolve};

use serde_json::Value;
use serde::{Deserialize};
use paho_mqtt as mqtt;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

pub struct MqttController {
    bind: Option<Bind>
}

struct Bind {
    config: Configuration,
    mqtt_config: MqttConfiguration,
    client: mqtt::Client,
    receiver: Receiver<Option<mqtt::Message>>
}

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

impl MqttController {
    pub fn new_empty() -> Self {
        return MqttController { bind: None };
    }
}

impl<'a> Controller<'a> for MqttController {
    fn init<'b: 'a>(&mut self, name: &str, config_json: &Value, paths: ModulePaths<'b>, dry_run: bool, no_docker: bool) -> Result<(), String> {
        let config: Configuration = conf_resolve!(config_json);
        let mqtt_config: MqttConfiguration = auth_resolve!(&config.auth_reference, &config.auth, paths.base_paths);

        let (client,receiver) : (mqtt::Client,Receiver<Option<mqtt::Message>>) =
            try_result!(get_client(&config, &mqtt_config), "Could not create mqtt client and receiver");

        self.bind = Some( Bind {
            config,
            mqtt_config,
            client,
            receiver
        });

        Ok(())
    }

    fn begin(&self) -> Result<bool, String> {
        let bound = try_option!(self.bind.as_ref(), "MQTT controller could not begin, as it is not bound");

        debug!("MQTT controller start run is beginning");
        info!("MQTT controller start run for device '{}' (start={})", bound.config.device, bound.config.start);

        let qos = bound.mqtt_config.qos;
        let topic_pub = get_topic_pub(&bound.config, &bound.mqtt_config);

        let result = start(&bound.client, &bound.receiver, bound.config.start, topic_pub, qos)?;

        debug!("MQTT controller start run is done");
        return Ok(result);
    }

    fn end(&self) -> Result<bool, String> {
        let bound = try_option!(self.bind.as_ref(), "MQTT controller could not end, as it is not bound");

        debug!("MQTT controller end run is beginning");
        info!("MQTT controller start run for device '{}' (start={})", bound.config.device, bound.config.start);

        let qos = bound.mqtt_config.qos;
        let topic_pub = get_topic_pub(&bound.config, &bound.mqtt_config);

        let result = try_result!(end(&bound.client, topic_pub, qos), "Could not end in mqtt controller");

        debug!("MQTT controller end run is done");
        return Ok(result);
    }

    fn clear(&mut self) -> Result<(), String> {
        let bound = try_option!(self.bind.as_ref(), "MQTT controller is not bound and thus can not be cleared");

        try_result!(bound.client.disconnect(None), "Disconnect from broker failed");

        self.bind = None;
        return Ok(());
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
    return bool_result!(client.publish(msg).is_ok(), true, "Could not send end message");
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

    // Set last will in case of whatever failure that includes a interrupted connection
    let testament_topic = get_topic_pub(config, mqtt_config);
    let testament = mqtt::Message::new(&testament_topic, "ABORT", mqtt_config.qos);
    options.will_message(testament);

    let topic_sub = get_topic_sub(config, mqtt_config);
    let qos = mqtt_config.qos;

    trace!("Subscription topic is '{}'", topic_sub);
    trace!("Publish topic is '{}'", testament_topic);

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
    config.topic_sub.clone().unwrap_or(
        format!("device/{}/controller/to/{}", config.device, mqtt_config.user))
}

fn get_topic_pub(config: &Configuration, mqtt_config: &MqttConfiguration) -> String {
    config.topic_pub.clone().unwrap_or(
        format!("device/{}/controller/from/{}", config.device, mqtt_config.user))
}