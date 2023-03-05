use crate::modules::traits::Controller;
use crate::util::objects::paths::ModulePaths;
use crate::util::io::json;
use crate::Arguments;
use crate::try_result;

use std::net::IpAddr;
use std::rc::Rc;
use serde_json::Value;
use serde::Deserialize;
use std::time::Duration;
use ping::ping;
use dns_lookup::lookup_host;

#[derive(Deserialize)]
pub struct DeserializedConfig {
    #[serde(rename = "address")]
    address: String,

    #[serde(default="default_timeout")]
    timeout: u64
}

pub struct Ping {
    ip_address: IpAddr,
    timeout: u64
}

fn default_timeout() -> u64 { 10 }

impl Controller for Ping {
    const MODULE_NAME: &'static str = "ping";

    fn new(_name: &str, config_json: &Value, _paths: ModulePaths, _args: &Rc<Arguments>) -> Result<Box<Self>, String> {
        let config = json::from_value::<DeserializedConfig>(config_json.clone())?; // TODO: - clone
        let ips: Vec<std::net::IpAddr> = try_result!(lookup_host(config.address.as_str()), format!("DNS lookup for '{}' failed", config.address));
        if ips.is_empty() {
            return Err(String::from("DNS lookup for '{}' found no addresses"));
        }

        return Ok(Box::new(Ping {
            ip_address: ips[0],
            timeout: config.timeout
        }));
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