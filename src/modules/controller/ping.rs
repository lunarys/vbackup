use crate::modules::traits::Controller;
use crate::util::objects::paths::ModulePaths;
use crate::util::io::json;
use crate::Arguments;

use std::net::IpAddr;
use serde_json::Value;
use serde::Deserialize;
use std::time::Duration;
use ping::ping;

#[derive(Deserialize)]
pub struct Ping {
    // TODO: should support domain names
    #[serde(rename = "address")]
    ip_address: IpAddr,

    #[serde(default="default_timeout")]
    timeout: u64
}

fn default_timeout() -> u64 { 10 }

impl Controller for Ping {
    const MODULE_NAME: &'static str = "ping";

    fn new(_name: &str, config_json: &Value, _paths: ModulePaths, _args: &Arguments) -> Result<Box<Self>, String> {
        let config = json::from_value::<Ping>(config_json.clone())?; // TODO: - clone
        return Ok(Box::new(config));
    }

    fn init(&mut self) -> Result<(), String> {
        return Ok(());
    }

    fn begin(&mut self) -> Result<bool, String> {
        info!("Trying to ping host '{}' with a timeout of {} seconds", self.ip_address, self.timeout);

        let result = ping(self.ip_address, Some(Duration::new(self.timeout, 0)), None, None, None, None);

        // ping just returns an error e.g. if ARP fails when there is no device with that IP (online)
        return Ok(result.is_ok());
    }

    fn end(&mut self) -> Result<bool, String> {
        return Ok(true);
    }

    fn clear(&mut self) -> Result<(), String> {
        return Ok(());
    }
}