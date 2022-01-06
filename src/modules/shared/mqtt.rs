use serde::{Deserialize};

fn default_qos() -> i32 { 1 }
fn default_port() -> i32 { 1883 }

#[derive(Deserialize)]
pub struct MqttConfiguration {
    pub host: String,

    #[serde(default="default_port")]
    pub port: i32,

    pub user: String,
    pub password: Option<String>,

    #[serde(default="default_qos")]
    pub qos: i32
}