use crate::modules::traits::{Controller, Bundleable};
use crate::util::io::{auth_data,json};
use crate::util::objects::paths::{Paths, ModulePaths};
use crate::Arguments;
use crate::{try_result,try_option,bool_result,dry_run};

use serde_json::Value;
use serde::{Deserialize};
use rumqtt::{MqttClient, MqttOptions, Receiver, Notification, SecurityOptions, QoS, Publish, LastWill};
use std::time::{Duration, Instant};
use std::rc::Rc;
use std::cmp::max;

pub struct MqttController {
    config: Configuration,
    mqtt_config: MqttConfiguration,
    dry_run: bool,
    name: String,
    paths: Rc<Paths>,
    connected: Option<Connection>
}

struct Connection {
    client: MqttClient,
    receiver: Receiver<Notification>,
    is_controller_online: bool
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
pub struct MqttConfiguration {
    pub host: String,

    #[serde(default="default_port")]
    pub port: u16,

    pub user: String,
    pub password: Option<String>,

    #[serde(default="default_qos")]
    pub qos: u8
}

fn default_qos() -> u8 { 1 }
fn default_port() -> u16 { 1883 }

impl Controller for MqttController {
    const MODULE_NAME: &'static str = "mqtt";

    fn new(name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Box<Self>, String> {
        return Bundleable::new_bundle(name, config_json, &paths.base_paths, args);
    }

    fn init(&mut self) -> Result<(), String> {
        let topic_pub = get_topic_pub(&self.config, &self.mqtt_config);
        let topic_sub = get_topic_sub(&self.config, &self.mqtt_config);
        trace!("Subscription topic is '{}'", topic_sub);
        trace!("Publish topic is '{}'", topic_pub);

        let qos = qos_from_u8(self.mqtt_config.qos)?;

        let (mut client,receiver) =
            try_result!(get_client(&self.mqtt_config, topic_pub.as_str(), "ABORT"), "Could not create mqtt client and receiver");
        try_result!(client.subscribe(&topic_sub, qos), "Could not subscribe to mqtt topic");

        let controller_topic = get_controller_state_topic(&self.config);
        let is_controller_online = get_controller_state(&mut client, &receiver, controller_topic, qos)?;
        if is_controller_online {
            debug!("MQTT controller for '{}' is available", self.config.device);
        } else {
            warn!("MQTT controller for '{}' is not available", self.config.device);
        }

        self.connected = Some(Connection {
            client,
            receiver,
            is_controller_online
        });

        return Ok(());
    }

    fn begin(&mut self) -> Result<bool, String> {
        let connection = try_option!(self.connected.as_mut(), "MQTT controller could not begin, as it is not connected... was the init step skipped?");

        info!("MQTT controller start run for device '{}' (start={})", self.config.device, self.config.start);

        if !connection.is_controller_online {
            return Ok(false);
        }

        let qos = qos_from_u8(self.mqtt_config.qos)?;
        let topic_pub = get_topic_pub(&self.config, &self.mqtt_config);

        let result = if !self.dry_run {
            start(&mut connection.client, &connection.receiver, self.config.start, topic_pub, qos)?
        } else {
            dry_run!(format!("Sending start command on MQTT topic '{}'", &topic_pub));
            true
        };

        trace!("MQTT controller start run is done");
        return Ok(result);
    }

    fn end(&mut self) -> Result<bool, String> {
        let connection = try_option!(self.connected.as_mut(), "MQTT controller could not end, as it is not connected... was the init step skipped?");

        debug!("MQTT controller end run for device '{}'", self.config.device);

        if !connection.is_controller_online {
            return Ok(false);
        }

        let qos = self.mqtt_config.qos;
        let topic_pub = get_topic_pub(&self.config, &self.mqtt_config);

        let result = if !self.dry_run {
            end(&mut connection.client, topic_pub, qos)?
        } else {
            dry_run!(format!("Sending end command on MQTT topic '{}'", &topic_pub));
            true
        };

        trace!("MQTT controller end run is done");
        return Ok(result);
    }

    fn clear(&mut self) -> Result<(), String> {
        let connection = try_option!(self.connected.as_mut(), "MQTT controller could not terminate, as it is not connected... was the init step skipped?");

        try_result!(connection.client.shutdown(), "Disconnect from broker failed");

        self.connected = None;
        return Ok(());
    }
}

impl Bundleable for MqttController {
    fn new_bundle(name: &str, config_json: &Value, paths: &Rc<Paths>, args: &Arguments) -> Result<Box<Self>, String> {
        let config = json::from_value::<Configuration>(config_json.clone())?; // TODO: - clone
        let mqtt_config = auth_data::resolve::<MqttConfiguration>(&config.auth_reference, &config.auth, paths)?;

        return Ok(Box::new(Self {
            config,
            mqtt_config,
            dry_run: args.dry_run,
            name: String::from(name),
            paths: paths.clone(),
            connected: None
        }));
    }

    fn try_bundle(&mut self, other_name: &str, other: &Value) -> Result<bool, String> {
        if self.connected.is_some() {
            error!("Can not bundle with a MQTT controller that is already connected");
            return Ok(false);
        }

        let other_config = json::from_value::<Configuration>(other.clone())?; // TODO: - clone
        let other_mqtt_config = auth_data::resolve::<MqttConfiguration>(&other_config.auth_reference, &other_config.auth, self.paths.as_ref())?;

        let result = self.config.device == other_config.device
            && self.config.topic_pub == other_config.topic_pub
            && self.config.topic_sub == other_config.topic_sub
            && self.mqtt_config.host == other_mqtt_config.host
            && self.mqtt_config.port == other_mqtt_config.port
            && self.mqtt_config.user == other_mqtt_config.user;

        if result && self.mqtt_config.password != other_mqtt_config.password {
            warn!("Password mismatch in otherwise bundleable MQTT controller configurations for '{}' and '{}'", self.name, other_name);
            return Ok(false);
        }

        if !result {
            return Ok(false);
        }

        self.config.start = self.config.start || other_config.start;
        self.mqtt_config.qos = max(self.mqtt_config.qos, other_mqtt_config.qos);

        return Ok(true);
    }
}

fn get_controller_state(client: &mut MqttClient, receiver: &Receiver<Notification>, topic: String, qos: QoS) -> Result<bool, String> {
    debug!("Checking the mqtt controller state for availability of the remote device");

    // Subscribe to state topic
    trace!("Subscribing to controller state topic");
    try_result!(client.subscribe(topic.as_str(), qos), "Could not subscribe to controller state topic");

    // 10 seconds should be more than enough, as the state is retained
    let wait_time = Duration::new(10,0);
    // Wait for a result
    let result_string = wait_for_message(receiver, Some(topic.as_str()), wait_time, None)?;
    let result = result_string.to_uppercase() == "ENABLED";

    // Unsubscribe from state topic
    trace!("Unsubscribing from controller state topic");
    try_result!(client.unsubscribe(topic.as_str()), "Could not unsubscribe from controller state topic");

    return Ok(result);
}

fn start(client: &mut MqttClient, receiver: &Receiver<Notification>, boot: bool, topic: String, qos: QoS) -> Result<bool, String> {
    let payload = if boot { "START_BOOT" } else { "START_RUN" };

    if let Err(err) = client.publish(topic.as_str(), qos, false, payload) {
        return Err(format!("Could not send start message: {}", err));
    }

    // TODO: Timeout as option?
    let timeout = Duration::new(600, 0);
    let received: String = wait_for_message(receiver, None, timeout, None)?;

    if received.to_lowercase().eq("disabled") {
        info!("Device is disabled and thus not available");
        return Ok(false);
    }

    // Device is already online
    if received.to_lowercase().eq("ready") {
        return Ok(true);
    }

    // Check only, do not boot, and device is offline
    if received.to_lowercase().eq("off") {
        return Ok(false);
    }

    // Wait until device is started
    if !(received.to_lowercase().eq("wait")) {
        return Err(format!("Expected to receive 'WAIT', but received '{}'", received));
    }

    // Second message should be CHECK
    let timeout2 = Duration::new(600, 0);
    let received2 = wait_for_message(receiver, None, timeout2, None)?;

    // Wait for check from controller to confirm still waiting
    if received2.to_lowercase().eq("check") {
        if client.publish(topic.as_str(), qos, false, "STILL_WAITING").is_err() {
            return Err(String::from("Could not send confirmation for still waiting"));
        }
    } else {
        return Err(format!("Expected to receive 'CHECK', but received '{}'", received2));
    }

    // Third message should just be confirmation with READY
    let timeout3 = Duration::new(600, 0);
    let received3 = wait_for_message(receiver, None, timeout3, None)?;

    // Return wether device is available or not
    return Ok(received3.to_lowercase().eq("ready"))
}

fn end(client: &mut MqttClient, topic: String, qos: u8) -> Result<bool, String> {
    let qos = try_result!(QoS::from_u8(qos), "Could not parse QoS value");
    let result = client.publish(topic, qos, false, "DONE");
    return bool_result!(result.is_ok(), true, "Could not send end message");
}

pub fn get_client(mqtt_config: &MqttConfiguration, testament_topic: &str, testament_payload: &str) -> Result<(MqttClient,Receiver<Notification>), String> {
    trace!("Trying to connect to mqtt broker with address '{}:{}'", mqtt_config.host.as_str(), mqtt_config.port);

    // TODO: id
    let mut options = MqttOptions::new("TODO: ID", mqtt_config.host.as_str(), mqtt_config.port);
    // options.set_reconnect_opts(mqtt::mqttoptions::ReconnectOptions::AfterFirstSuccess(15));
    // options.set_connection_timeout(30);
    options = options.set_clean_session(true);

    // Set last will in case of whatever failure that includes a interrupted connection
    let testament_topic = String::from(testament_topic);
    let qos = try_result!(QoS::from_u8(mqtt_config.qos), "Could not parse QoS value");
    let last_will = LastWill {
        topic: testament_topic,
        message: String::from(testament_payload),
        qos,
        retain: false
    };
    options = options.set_last_will(last_will);

    // set authentication
    if mqtt_config.password.is_some() {
        let auth = SecurityOptions::UsernamePassword(mqtt_config.user.clone(), mqtt_config.password.clone().unwrap());
        options = options.set_security_opts(auth);
    }

    return MqttClient::start(options).map_err(|e| format!("Could not connect to the mqtt broker: {}", e));
}

fn wait_for_message(receiver: &Receiver<Notification>, on_topic: Option<&str>, timeout: Duration, expected: Option<String>) -> Result<String, String> {
    let start_time = Instant::now();

    loop {
        let time_left = timeout - start_time.elapsed();
        let received_message = try_option!(wait_for_publish(receiver, time_left), "Timeout on receive operation");
        let payload = decode_payload(&received_message.payload)?;

        debug!("Received mqtt message '{}'", payload);

        if let Some(expected_topic) = on_topic {
            trace!("Expected to receive message on '{}', received on '{}'", received_message.topic_name, expected_topic);
            if received_message.topic_name != expected_topic {
                debug!("Received message on topic other than the expected one, still waiting");
                continue;
            }
        }

        if let Some(expected_string) = expected.as_ref() {
            if payload.eq(expected_string.as_str()) {
                return Ok(payload);
            }
        } else {
            return Ok(payload);
        }

        // Did not receive expected message -> Wait again
    }
}

fn wait_for_publish(receiver: &Receiver<Notification>, timeout: Duration) -> Option<Publish> {
    let start_time = Instant::now();
    loop {
        let time_left = timeout - start_time.elapsed();

        if let Ok(notification) = receiver.recv_timeout(time_left) {
            // TODO: Reconnect on connection loss
            if let Notification::Publish(publish) = notification {
                return Some(publish);
            }

            // TODO: ??
            //  - None
            //  - Other variants
            //  Currently: loop continues
        } else {
            return None;
        }

        //let received = try_result!(receiver.recv_timeout(time_left), "Timeout on receive operation");
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

fn get_controller_state_topic(config: &Configuration) -> String {
    return format!("device/{}/controller/status", config.device);
}

pub fn decode_payload(payload: &Vec<u8>) -> Result<String,String> {
    return std::str::from_utf8(payload)
        .map(|s| s.to_owned())
        .map_err(|e| format!("Could not decode payload as UTF-8: {}", e));
}

pub fn qos_from_u8(input: u8) -> Result<QoS, String> {
    return QoS::from_u8(input).map_err(|e| format!("Could not parse QoS: {}", e));
}