use crate::modules::traits::Reporting;
use crate::util::objects::reporting::*;
use crate::util::io::{auth_data,json};
use crate::util::objects::paths::{Paths};
use crate::modules::shared::mqtt::MqttConfiguration;
use crate::Arguments;
use crate::{try_result, try_result_debug};
use crate::modules::controller::mqtt::{get_client,qos_from_u8};

use serde_json::Value;
use serde::{Deserialize};
use std::ops::AddAssign;
use std::rc::Rc;
use std::thread::JoinHandle;
use rumqttc::{Client};

pub struct Reporter {
    config: Configuration,
    mqtt_config: MqttConfiguration,
    client: Option<Client>,
    join_handle: Option<JoinHandle<()>>
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
            client: None,
            join_handle: None
        }));
    }

    fn init(&mut self) -> Result<(), String> {
        let base_topic = get_base_topic(&self.config, &self.mqtt_config);
        trace!("Base topic is '{}'", base_topic);

        let (client,_,join_handle) = try_result!(get_client(&self.mqtt_config, base_topic.as_str(), "cancelled", None), "Could not create mqtt client and receiver");
        self.client = Some(client);
        self.join_handle = Some(join_handle);

        return Ok(());
    }

    fn report(&mut self, event: ReportEvent) -> Result<(), String> {
        if !self.client.is_some() {
            return Err(String::from("MQTT reporter is not connected for reporting"));
        }

        let qos = qos_from_u8(self.mqtt_config.qos)?;
        let mut topic = get_base_topic(&self.config, &self.mqtt_config);

        let message = match event {
            ReportEvent::Version(version) => {
                topic.push_str("/version");
                version
            },
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
            ReportEvent::Operation(operation) => {
                match operation {
                    OperationStatus::START(op) => op,
                    OperationStatus::DONE => String::from("done")
                }
            }
        };

        // TODO: Trace or debug?
        trace!("Reporting on '{}': '{}'", topic.as_str(), message);

        if let Err(err) = self.client.as_mut().unwrap().publish(topic, qos, false, message) {
            return Err(format!("Could not send report: {}", err));
        }

        // Return whether device is available or not
        return Ok(());
    }

    fn clear(&mut self) -> Result<(), String> {
        if let Some(mut client) = self.client.take() {
            try_result!(client.disconnect(), "Disconnect from broker failed");

            if let Some(join_handle) = self.join_handle.take() {
                try_result_debug!(join_handle.join(), "Error when trying to wait for the MQTT thread");
            }
        }

        return Ok(());
    }
}

fn get_base_topic(config: &Configuration, mqtt_config: &MqttConfiguration) -> String {
    config.base_topic.clone().unwrap_or(format!("device/{}/vbackup", mqtt_config.user))
}