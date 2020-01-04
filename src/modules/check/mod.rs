pub enum CheckModule {
    Unknown
}

use CheckModule::*;

pub fn get_module(name: &str) -> Result<CheckModule, String> {
    return Ok(match name.to_lowercase().as_str() {
        //"mqtt" => MQTT(mqtt::MqttController::new_empty()),
        unknown => {
            let msg = format!("Unknown controller module: '{}'", unknown);
            error!("{}", msg);
            return Err(msg)
        }
    })
}