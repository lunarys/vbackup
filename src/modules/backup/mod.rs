pub enum BackupModule {
    Unknown
}

use BackupModule::*;

pub fn get_module(name: &str) -> Result<BackupModule, String> {
    return Ok(match name.to_lowercase().as_str() {
        //"mqtt" => MQTT(mqtt::MqttController::new_empty()),
        unknown => {
            let msg = format!("Unknown controller module: '{}'", unknown);
            error!("{}", msg);
            return Err(msg)
        }
    })
}