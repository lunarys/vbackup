use serde::{Deserialize};
use crate::{try_result,try_option,log_error};
use rumqttc::{Client, MqttOptions, QoS, Publish, LastWill, Event, Packet, Outgoing};
use std::time::{Duration, Instant};
use std::thread;
use crossbeam_channel::{unbounded, Receiver, bounded};
use std::thread::{JoinHandle};
use rand;

fn default_qos() -> u8 { 1 }
fn default_port() -> u16 { 1883 }
fn default_false() -> bool { false }
fn default_timeout() -> u64 { 15 }

#[derive(Deserialize)]
pub struct MqttConfiguration {
    pub host: String,

    #[serde(default="default_port")]
    pub port: u16,

    pub user: String,
    pub password: Option<String>,

    #[serde(default="default_qos")]
    pub qos: u8,

    #[serde(default="default_false")]
    pub retain: bool,

    #[serde(default="default_timeout")]
    pub connect_timeout_sec: u64
}

pub fn get_client(mqtt_config: &MqttConfiguration, testament_topic: &str, testament_payload: &str, auto_subscibe: Option<Vec<String>>) -> Result<(Client,Receiver<Publish>,JoinHandle<()>), String> {
    trace!("Trying to connect to mqtt broker with address '{}:{}'", mqtt_config.host.as_str(), mqtt_config.port);

    let random_id: i32 = rand::random();
    let mqtt_client_id = format!("vbackup-{}-{}", mqtt_config.user, random_id);

    let mut options = MqttOptions::new(mqtt_client_id, mqtt_config.host.as_str(), mqtt_config.port);
    // options.set_reconnect_opts(mqtt::mqttoptions::ReconnectOptions::AfterFirstSuccess(15));
    // options.set_connection_timeout(30);
    options.set_clean_session(true);

    // Set last will in case of whatever failure that includes a interrupted connection
    let testament_topic = String::from(testament_topic);
    let qos = try_result!(rumqttc::qos(mqtt_config.qos), "Could not parse QoS value");
    let last_will = LastWill::new(testament_topic, testament_payload, qos, mqtt_config.retain);

    options.set_last_will(last_will);

    // set authentication
    if mqtt_config.password.is_some() {
        options.set_credentials(mqtt_config.user.clone(), mqtt_config.password.clone().unwrap());
    }

    // TODO: cap=10 ?? (taken from crate examples)
    let (client,mut connection) = Client::new(options, 10);
    let mut client_clone = client.clone(); // move this to the mqtt event loop in order to (re)subscribe
    let qos = qos_from_u8(mqtt_config.qos)?;

    // create a channel that received messages are sent into, such that they can be received by the main thread
    let (sender, receiver) = unbounded();
    let (thread_sender, thread_receiver) = bounded(1);

    let handle = thread::spawn(move || {
        let mut did_connect = false;
        let mut error_count = 0;

        for (_i, notification) in connection.iter().enumerate() {
            match notification {
                Ok(event) => {
                    match event {
                        Event::Incoming(packet) => {
                            match packet {
                                Packet::Publish(publish) => {
                                    // TODO: if there is an error act accordingly?
                                    log_error!(sender.send(publish));
                                },
                                Packet::Disconnect => {
                                    // if not terminated from outgoing disconnect, terminate now
                                    break;
                                },
                                Packet::ConnAck(conn_ack) => {
                                    if did_connect {
                                        info!("Reconnected to mqtt broker");
                                    } else {
                                        debug!("Connected to mqtt broker");
                                    }

                                    if let Some(subscribe) = auto_subscibe.as_ref() {
                                        subscribe.iter().for_each(|topic| {
                                            let result = client_clone.subscribe(topic, qos);
                                            if result.is_err() {
                                                error!("Could not automatically subscribe to mqtt topic '{}' after connect", topic);
                                            }
                                        });
                                    }

                                    // only notify the parent thread about the connection once at the beginning
                                    if !did_connect {
                                        did_connect = true;
                                        // TODO: if there is an error act accordingly?
                                        log_error!(thread_sender.send(conn_ack));
                                    }
                                },
                                _ => {}
                            }
                        }
                        Event::Outgoing(packet) => {
                            match packet {
                                Outgoing::Disconnect => {
                                    // Terminate the receiver loop (and thus thread) on disconnect
                                    // Sleep for a short time to make sure disconnect is properly done
                                    // Otherwise the last will might get triggered
                                    // TODO: find a better way to handle this...
                                    // Also used in this issue: https://github.com/bytebeamio/rumqtt/issues/662#issuecomment-1646586799
                                    thread::sleep(Duration::from_millis(500));
                                    break;
                                },
                                _ => {}
                            }
                        }
                    }
                },
                Err(error) => {
                    error!("Connection error ({}) in mqtt receiver: {}", { error_count += 1; error_count }, error);

                    // sleep after an error before triggering a reconnect through the event loop
                    thread::sleep(Duration::from_secs(5))
                }
            }
        }
    });

    let timeout_secs = mqtt_config.connect_timeout_sec;

    // await connect?
    try_result!(thread_receiver.recv_timeout(Duration::from_secs(timeout_secs)), format!("Could not connect to the mqtt broker within {} seconds", timeout_secs));

    return Ok((client, receiver, handle));
}

pub fn wait_for_message(receiver: &Receiver<Publish>, on_topic: Option<&str>, timeout: Duration, expected: Option<String>) -> Result<String, String> {
    let start_time = Instant::now();

    loop {
        let time_left = timeout - start_time.elapsed();

        let received_message = try_option!(wait_for_publish(receiver, time_left), "Timeout on receive operation");
        let payload = decode_payload(received_message.payload.as_ref())?;

        debug!("Received mqtt message '{}'", payload);

        if let Some(expected_topic) = on_topic {
            trace!("Expected to receive message on '{}', received on '{}'", &received_message.topic, expected_topic);
            if received_message.topic.as_str() != expected_topic {
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

pub fn wait_for_publish(receiver: &Receiver<Publish>, timeout: Duration) -> Option<Publish> {
    let start_time = Instant::now();
    loop {
        let time_left = timeout - start_time.elapsed();

        return if let Ok(notification) = receiver.recv_timeout(time_left) {
            Some(notification)
        } else {
            None
        }
    }
}

pub fn decode_payload(payload: &[u8]) -> Result<String,String> {
    return std::str::from_utf8(payload)
        .map(|s| s.to_owned())
        .map_err(|e| format!("Could not decode payload as UTF-8: {}", e));
}

pub fn qos_from_u8(input: u8) -> Result<QoS, String> {
    return rumqttc::qos(input).map_err(|e| format!("Could not parse QoS: {}", e));
}
