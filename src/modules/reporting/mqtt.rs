use crate::modules::traits::Reporting;
use crate::util::io::{auth_data,json};
use crate::modules::object::{Paths,Arguments};

use crate::{try_result,try_option};

use serde_json::Value;
use serde::{Deserialize};
use paho_mqtt as mqtt;
use std::ops::AddAssign;

pub struct Reporter {
    bind: Option<Bind>
}

struct Bind {
    config: Configuration,
    mqtt_config: MqttConfiguration,
    client: mqtt::Client
}

#[derive(Deserialize)]
struct Configuration {
    auth_reference: Option<String>,
    base_topic: Option<String>,
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

impl Reporter {
    pub fn new_empty() -> Self {
        return Reporter{ bind: None };
    }
}

impl Reporting for Reporter {
    fn init(&mut self, config_json: &Value, paths: &Paths, _args: &Arguments) -> Result<(), String> {
        if self.bind.is_some() {
            let msg = String::from("Reporting module is already bound");
            error!("{}", msg);
            return Err(msg);
        }

        let config = json::from_value::<Configuration>(config_json.clone())?; // TODO: - clone
        let mqtt_config = auth_data::resolve::<MqttConfiguration>(&config.auth_reference, &config.auth, paths)?;

        let client: mqtt::Client =
            try_result!(get_client(&config, &mqtt_config), "Could not create mqtt client and receiver");

        self.bind = Some(Bind {
            config,
            mqtt_config,
            client
        });

        return Ok(());
    }

    fn report(&self, context: Option<&[&str]>, value: &str) -> Result<(), String> {
        let bound = try_option!(self.bind.as_ref(), "MQTT reporter is not bound");

        let qos = bound.mqtt_config.qos;
        let mut topic = get_base_topic(&bound.config, &bound.mqtt_config);

        match context {
            // Size reporting for specific operation on specific volume
            Some([_, name, "size", what]) => {
                topic.push('/');
                topic.add_assign(name);
                topic.push('/');
                topic.add_assign(what);
                topic.push('/');
                topic.add_assign("size");
            },
            // Accumulated size reporting
            Some(["size", what]) => {
                topic.push('/');
                topic.add_assign("size");
                topic.push('/');
                topic.add_assign(what);
            },
            // General activity reporting
            Some([operation, name]) => {
                topic.push('/');
                topic.add_assign(name);
                topic.push('/');
                topic.add_assign(operation);
            },
            _ => {},
        }

        // TODO: Trace or debug?
        trace!("Reporting on '{}': '{}'", topic.as_str(), value);

        let message = String::from(value);

        let msg = mqtt::Message::new(topic, message, qos);

        if bound.client.publish(msg).is_err() {
            let msg = String::from("Could not send report");
            error!("{}", msg);
            return Err(String::from(msg));
        }

        // Return wether device is available or not
        Ok(())
    }

    fn clear(&mut self) -> Result<(), String> {
        let bound = try_option!(self.bind.as_ref(), "MQTT reporter is not bound");

        try_result!(bound.client.disconnect(None), "Disconnect from broker failed");

        self.bind = None;
        return Ok(());
    }
}

fn get_client(config: &Configuration, mqtt_config: &MqttConfiguration) -> Result<mqtt::Client, String> {
    let mqtt_host = format!("tcp://{}:{}", mqtt_config.host, mqtt_config.port);

    trace!("Trying to connect to mqtt broker with address '{}'", mqtt_host);

    let client: mqtt::Client = try_result!(mqtt::Client::new(mqtt_host), "Failed connecting to broker");

    let mut options_builder = mqtt::ConnectOptionsBuilder::new();
    let mut options = options_builder.clean_session(true);
    options = options.user_name(mqtt_config.user.as_str());
    if mqtt_config.password.is_some() {
        options = options.password(mqtt_config.password.as_ref().unwrap().as_str());
    }

    //options.connect_timeout()
    //options.automatic_reconnect()

    // Set last will in case of whatever failure that includes a interrupted connection
    let testament_topic = get_base_topic(config, mqtt_config);
    let testament = mqtt::Message::new(testament_topic.as_str(), "cancelled", mqtt_config.qos);
    options.will_message(testament);

    trace!("Base topic is '{}'", testament_topic);
    try_result!(client.connect(options.finalize()), "Could not connect to the mqtt broker");

    Ok(client)
}

fn get_base_topic(config: &Configuration, mqtt_config: &MqttConfiguration) -> String {
    config.base_topic.clone().unwrap_or(format!("device/{}/vbackup", mqtt_config.user))
}