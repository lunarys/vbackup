use crate::modules::traits::Reporting;
use crate::util::objects::reporting::*;
use crate::util::io::{auth_data,json};
use crate::util::objects::paths::{Paths};
use crate::util::objects::shared::mqtt::MqttConfiguration;
use crate::Arguments;
use crate::{try_result};

use serde_json::Value;
use serde::{Deserialize};
use paho_mqtt as mqtt;
use std::ops::AddAssign;
use std::rc::Rc;

pub struct Reporter {
    config: Configuration,
    mqtt_config: MqttConfiguration,
    client: Option<mqtt::Client>
}

#[derive(Deserialize)]
struct Configuration {
    auth_reference: Option<String>,
    base_topic: Option<String>,
    auth: Option<Value>
}

impl Reporting for Reporter {
    const MODULE_NAME: &'static str = "mqtt";

    fn new(config_json: &Value, paths: &Rc<Paths>, _args: &Arguments) -> Result<Box<Self>, String> {
        let config = json::from_value::<Configuration>(config_json.clone())?; // TODO: - clone
        let mqtt_config = auth_data::resolve::<MqttConfiguration>(&config.auth_reference, &config.auth, paths)?;

        return Ok(Box::new(Reporter {
            config,
            mqtt_config,
            client: None
        }));
    }

    fn init(&mut self) -> Result<(), String> {
        let client: mqtt::Client = try_result!(get_client(&self.config, &self.mqtt_config), "Could not create mqtt client and receiver");
        self.client = Some(client);

        return Ok(());
    }

    fn report(&self, event: ReportEvent) -> Result<(), String> {
        if !self.client.is_some() {
            return Err(String::from("MQTT reporter is not connected for reporting"));
        }

        let qos = self.mqtt_config.qos;
        let mut topic = get_base_topic(&self.config, &self.mqtt_config);

        let message = match event {
            ReportEvent::Status(report) => {
                if let Some(name) = report.module {
                    topic.push('/');
                    topic.add_assign(name.as_str());
                    topic.push('/');
                    topic.add_assign(match report.run_type {
                        RunType::RUN => "run",
                        RunType::BACKUP => "backup",
                        RunType::SYNC => "sync"
                    });
                }

                String::from(match report.status {
                    Status::START => "starting",
                    Status::DONE => "done",
                    Status::ERROR => "failed",
                    Status::SKIP => "skipped",
                    Status::DISABLED => "disabled"
                })
            },
            ReportEvent::Size(report) => {
                if let Some(name) = report.module {
                    topic.push('/');
                    topic.add_assign(name.as_str());
                }

                topic.push('/');
                topic.add_assign("size");

                topic.push('/');
                topic.add_assign(match report.size_type {
                    SizeType::ORIGINAL => "original",
                    SizeType::BACKUP => "backup",
                    SizeType::SYNC => "synced"
                });

                report.size.to_string()
            }
            ReportEvent::Operation(operation) => {
                match operation {
                    OperationStatus::START(op) => op,
                    OperationStatus::DONE => String::from("done")
                }
            }
        };

        // TODO: Trace or debug?
        trace!("Reporting on '{}': '{}'", topic.as_str(), message);

        let msg = mqtt::Message::new(topic, message, qos);

        if self.client.as_ref().unwrap().publish(msg).is_err() {
            return Err(String::from("Could not send report"));
        }

        // Return wether device is available or not
        return Ok(());
    }

    fn clear(&mut self) -> Result<(), String> {
        if let Some(client) = self.client.as_ref() {
            try_result!(client.disconnect(None), "Disconnect from broker failed");
        }

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