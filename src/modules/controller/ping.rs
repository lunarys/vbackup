use crate::modules::traits::Controller;
use crate::util::objects::paths::ModulePaths;
use crate::util::io::json;
use crate::Arguments;

use std::net::IpAddr;
use serde_json::Value;
use serde::Deserialize;
use std::time::Duration;
use ping::ping;

pub struct Ping {
    bind: Option<PingConfig>
}

#[derive(Deserialize)]
struct PingConfig {
    // TODO: should support domain names
    #[serde(rename = "address")]
    ip_address: IpAddr,

    #[serde(default="default_timeout")]
    timeout: u64
}

fn default_timeout() -> u64 { 10 }

impl Ping {
    pub fn new_empty() -> Self { Ping { bind: None } }

    fn error_if_not_bound(&mut self) -> Result<(),String> {
        if self.bind.is_none() {
            let msg = String::from("Controller module is not bound");
            error!("{}", msg);
            return Err(msg);
        }

        return Ok(());
    }
}

impl Controller for Ping {
    fn init(&mut self, _name: &str, config_json: &Value, _paths: ModulePaths, _args: &Arguments) -> Result<(), String> {
        if self.bind.is_some() {
            let msg = String::from("Controller module is already bound");
            error!("{}", msg);
            return Err(msg);
        }

        let config = json::from_value::<PingConfig>(config_json.clone())?; // TODO: - clone
        self.bind = Some(config);

        return Ok(());
    }

    fn begin(&mut self) -> Result<bool, String> {
        if let Some(bound) = self.bind.as_ref() {
            info!("Trying to ping host '{}' with a timeout of {} seconds", bound.ip_address, bound.timeout);

            let result = ping(bound.ip_address, Some(Duration::new(bound.timeout, 0)), None, None, None, None);

            // ping just returns an error e.g. if ARP fails when there is no device with that IP (online)
            return Ok(result.is_ok());
        } else {
            return self.error_if_not_bound().map(|_| false);
        }
    }

    fn end(&mut self) -> Result<bool, String> {
        return self.error_if_not_bound().map(|_| true);
    }

    fn clear(&mut self) -> Result<(), String> {
        self.error_if_not_bound()?;
        self.bind = None;
        return Ok(());
    }
}