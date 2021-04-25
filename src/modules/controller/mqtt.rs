use crate::modules::traits::{Controller, Bundleable};
use crate::util::io::{auth_data,json};
use crate::util::objects::paths::{Paths, ModulePaths};
use crate::util::objects::shared::mqtt::MqttConfiguration;
use crate::Arguments;
use crate::{try_result,try_option,bool_result,dry_run};

use serde_json::Value;
use serde::{Deserialize};
use paho_mqtt as mqtt;
use std::sync::mpsc::Receiver;
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
    client: mqtt::Client,
    receiver: Receiver<Option<mqtt::Message>>,
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

impl Controller for MqttController {
    const MODULE_NAME: &'static str = "mqtt";

    fn new(name: &str, config_json: &Value, paths: ModulePaths, args: &Arguments) -> Result<Box<Self>, String> {
        return Bundleable::new_bundle(name, config_json, &paths.base_paths, args);
    }

    fn init(&mut self) -> Result<(), String> {
        let (client,receiver) : (mqtt::Client,Receiver<Option<mqtt::Message>>) =
            try_result!(get_client(&self.config, &self.mqtt_config), "Could not create mqtt client and receiver");

        let controller_topic = get_controller_state_topic(&self.config);
        let is_controller_online = get_controller_state(&client, &receiver, controller_topic, self.mqtt_config.qos)?;
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
        let connection = try_option!(self.connected.as_ref(), "MQTT controller could not begin, as it is not connected... was the init step skipped?");

        info!("MQTT controller start run for device '{}' (start={})", self.config.device, self.config.start);

        if !connection.is_controller_online {
            return Ok(false);
        }

        let qos = self.mqtt_config.qos;
        let topic_pub = get_topic_pub(&self.config, &self.mqtt_config);

        let result = if !self.dry_run {
            start(&connection.client, &connection.receiver, self.config.start, topic_pub, qos)?
        } else {
            dry_run!(format!("Sending start command on MQTT topic '{}'", &topic_pub));
            true
        };

        trace!("MQTT controller start run is done");
        return Ok(result);
    }

    fn end(&mut self) -> Result<bool, String> {
        let connection = try_option!(self.connected.as_ref(), "MQTT controller could not end, as it is not connected... was the init step skipped?");

        info!("MQTT controller end run for device '{}'", self.config.device);

        if !connection.is_controller_online {
            return Ok(false);
        }

        let qos = self.mqtt_config.qos;
        let topic_pub = get_topic_pub(&self.config, &self.mqtt_config);

        let result = if !self.dry_run {
            end(&connection.client, topic_pub, qos)?
        } else {
            dry_run!(format!("Sending end command on MQTT topic '{}'", &topic_pub));
            true
        };

        trace!("MQTT controller end run is done");
        return Ok(result);
    }

    fn clear(&mut self) -> Result<(), String> {
        let connection = try_option!(self.connected.as_ref(), "MQTT controller could not terminate, as it is not connected... was the init step skipped?");

        try_result!(connection.client.disconnect(None), "Disconnect from broker failed");

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

fn get_controller_state(client: &mqtt::Client, receiver: &Receiver<Option<mqtt::Message>>, topic: String, qos: i32) -> Result<bool, String> {
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

fn start(client: &mqtt::Client, receiver: &Receiver<Option<mqtt::Message>>, boot: bool, topic: String, qos: i32) -> Result<bool, String> {
    let msg = mqtt::Message::new(topic.as_str(), if boot { "START_BOOT" } else { "START_RUN" }, qos);

    if client.publish(msg).is_err() {
        return Err("Could not send start initiation message".to_string());
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
        let msg = mqtt::Message::new(topic.as_str(), "STILL_WAITING", qos);
        if client.publish(msg).is_err() {
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
    options = options.user_name(mqtt_config.user.as_str());
    if mqtt_config.password.is_some() {
        options = options.password(mqtt_config.password.as_ref().unwrap().as_str());
    }

    //options.connect_timeout()
    //options.automatic_reconnect()

    // Set last will in case of whatever failure that includes a interrupted connection
    let testament_topic = get_topic_pub(config, mqtt_config);
    let testament = mqtt::Message::new(testament_topic.as_str(), "ABORT", mqtt_config.qos);
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

fn wait_for_message(receiver: &Receiver<Option<mqtt::Message>>, on_topic: Option<&str>, timeout: Duration, expected: Option<String>) -> Result<String, String> {
    let start_time = Instant::now();

    loop {
        let time_left = timeout - start_time.elapsed();

        let received : Option<mqtt::Message> = try_result!(receiver.recv_timeout(time_left), "Timeout exceeded");
        // TODO: What was this again?
        let received_message: mqtt::Message = try_option!(received, "Timeout on receive operation");
        // TODO: Reconnect on connection loss

        debug!("Received mqtt message '{}'", received_message.to_string());

        if let Some(expected_topic) = on_topic {
            trace!("Expected to receive message on '{}', received on '{}'", received_message.topic(), expected_topic);
            if received_message.topic() != expected_topic {
                debug!("Received message on topic other than the expected one, still waiting");
                continue;
            }
        }

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

fn get_controller_state_topic(config: &Configuration) -> String {
    return format!("device/{}/controller/status", config.device);
}