use crate::modules::traits::{Controller, Bundleable};
use crate::util::io::{auth_data,json};
use crate::util::objects::paths::{Paths, ModulePaths};
use crate::modules::shared::mqtt::{MqttConfiguration, qos_from_u8, wait_for_message, get_client};
use crate::Arguments;
use crate::{try_result,try_option,bool_result,dry_run,try_result_debug};

use serde_json::Value;
use serde::{Deserialize};
use rumqttc::{Client, QoS, Publish};
use std::time::{Duration};
use std::rc::Rc;
use std::cmp::max;
use std::thread::JoinHandle;
use crossbeam_channel::{Receiver};

pub struct MqttController {
    config: Configuration,
    mqtt_config: MqttConfiguration,
    args: Rc<Arguments>,
    name: String,
    paths: Rc<Paths>,
    connected: Option<MqttConnection>
}

struct MqttConnection {
    client: Client,
    receiver: Receiver<Publish>,
    is_controller_online: bool,
    join_handle: JoinHandle<()>
}

#[derive(Deserialize)]
struct Configuration {
    start: bool,
    device: String,
    auth_reference: Option<String>,
    topic_sub: Option<String>,
    topic_pub: Option<String>,
    auth: Option<Value>,

    #[serde(default="default_timeout_start")]
    start_timeout_sec: u64,

    #[serde(default="default_timeout_controller")]
    controller_timeout_sec: u64
}

fn default_timeout_start() -> u64 { 600 }
fn default_timeout_controller() -> u64 { 10 }

impl Controller for MqttController {
    const MODULE_NAME: &'static str = "mqtt";

    fn new(name: &str, config_json: &Value, paths: ModulePaths, args: &Rc<Arguments>) -> Result<Box<Self>, String> {
        return Bundleable::new_bundle(name, config_json, &paths.base_paths, args);
    }

    fn init(&mut self) -> Result<(), String> {
        let topic_pub = get_topic_pub(&self.config, &self.mqtt_config);
        let topic_sub = get_topic_sub(&self.config, &self.mqtt_config);
        trace!("Subscription topic is '{}'", topic_sub);
        trace!("Publish topic is '{}'", topic_pub);

        let qos = qos_from_u8(self.mqtt_config.qos)?;

        let (mut client,receiver, join_handle) = try_result!(get_client(&self.mqtt_config, topic_pub.as_str(), "ABORT", Some(vec![topic_sub.clone()])), "Could not create mqtt client and receiver");

        let controller_topic = get_controller_state_topic(&self.config);
        let is_controller_online = get_controller_state(&mut client, &receiver, controller_topic, qos, self.config.controller_timeout_sec)?;
        if is_controller_online {
            debug!("MQTT controller for '{}' is available", self.config.device);
        } else {
            warn!("MQTT controller for '{}' is not available", self.config.device);
        }

        self.connected = Some(MqttConnection {
            client,
            receiver,
            is_controller_online,
            join_handle
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

        let result = if !self.args.dry_run {
            start(&mut connection.client, &connection.receiver, self.config.start, topic_pub, qos, self.config.start_timeout_sec)?
        } else {
            dry_run!(format!("Sending start command on MQTT topic '{}'", &topic_pub));
            true
        };

        trace!("MQTT controller start run is done");
        return Ok(result);
    }

    fn end(&mut self) -> Result<bool, String> {
        if let Some(connection) = self.connected.as_mut() {
            info!("MQTT controller end run for device '{}'", self.config.device);

            if !connection.is_controller_online {
                return Ok(false);
            }

            let qos = self.mqtt_config.qos;
            let topic_pub = get_topic_pub(&self.config, &self.mqtt_config);

            let result = if !self.args.dry_run {
                end(&mut connection.client, topic_pub, qos)?
            } else {
                dry_run!(format!("Sending end command on MQTT topic '{}'", &topic_pub));
                true
            };

            trace!("MQTT controller end run is done");
            Ok(result)
        } else {
            debug!("MQTT controller was not connected, skipping end procedure");
            Ok(false)
        }
    }

    fn clear(&mut self) -> Result<(), String> {
        if let Some(mut connection) = self.connected.take() {
            try_result!(connection.client.disconnect(), "Disconnect from broker failed");
            try_result_debug!(connection.join_handle.join(), "Error when trying to wait for the MQTT thread");
            Ok(())
        } else {
            trace!("MQTT controller was not connected, skipping disconnect");
            Ok(())
        }
    }
}

impl Bundleable for MqttController {
    fn new_bundle(name: &str, config_json: &Value, paths: &Rc<Paths>, args: &Rc<Arguments>) -> Result<Box<Self>, String> {
        let config = json::from_value::<Configuration>(config_json.clone())?; // TODO: - clone
        let mqtt_config = auth_data::resolve::<MqttConfiguration>(&config.auth_reference, &config.auth, paths)?;

        return Ok(Box::new(Self {
            config,
            mqtt_config,
            args: args.clone(),
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

fn get_controller_state(client: &mut Client, receiver: &Receiver<Publish>, topic: String, qos: QoS, timeout_sec: u64) -> Result<bool, String> {
    debug!("Checking the mqtt controller state for availability of the remote device");

    // Subscribe to state topic
    trace!("Subscribing to controller state topic");
    try_result!(client.subscribe(topic.as_str(), qos), "Could not subscribe to controller state topic");

    // 10 seconds should be more than enough, as the state is retained
    let wait_time = Duration::from_secs(timeout_sec);
    // Wait for a result
    let result_string = try_result!(wait_for_message(receiver, Some(topic.as_str()), wait_time, None), "Mqtt controller state check did not respond");
    let result = result_string.to_uppercase() == "ENABLED";

    // Unsubscribe from state topic
    trace!("Unsubscribing from controller state topic");
    try_result!(client.unsubscribe(topic.as_str()), "Could not unsubscribe from controller state topic");

    return Ok(result);
}

fn start(client: &mut Client, receiver: &Receiver<Publish>, boot: bool, topic: String, qos: QoS, timeout_sec: u64) -> Result<bool, String> {
    let payload = if boot { "START_BOOT" } else { "START_RUN" };

    if let Err(err) = client.publish(topic.as_str(), qos, false, payload) {
        return Err(format!("Could not send start message: {}", err));
    }

    let timeout = Duration::from_secs(timeout_sec);
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
    let timeout2 = Duration::from_secs(timeout_sec);
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

fn end(client: &mut Client, topic: String, qos: u8) -> Result<bool, String> {
    let qos = try_result!(rumqttc::qos(qos), "Could not parse QoS value");
    let result = client.publish(topic, qos, false, "DONE");
    return bool_result!(result.is_ok(), true, "Could not send end message");
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
