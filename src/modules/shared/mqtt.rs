use serde::{Deserialize};

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