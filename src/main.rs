#[macro_use]
extern crate log;
extern crate env_logger;
extern crate serde_json;
extern crate serde;

extern crate serde_derive;

mod vbackup;
mod modules;
mod util;

use log::LevelFilter;
use env_logger::Builder;

use crate::modules::traits::Sync;
use crate::modules::traits::Controller;
use crate::modules::object::{Paths, PathBase};

fn main() {
    Builder::new()
        .filter_level(LevelFilter::Trace)
        .filter_module("paho_mqtt", LevelFilter::Error)
        .init();

    info!("Hello, world!");

    //modules::sync::get_module_list().get("duplicati").unwrap().sync();

    let controller_config = r#"
       {
            "start": true,
            "device": "sundavar",
            "auth": {
                "user": "ju",
                "password": "testpass",
                "host": "localhost",
                "port": 1883
            }
       }
    "#;

    let duplicati_config = r#"
        {
            "encryption_key": "secret",
            "directory": "directory",
            "auth": {
                "hostname": "sundavar.elda",
                "port": 987,
                "user": "ju",
                "password": "pass",
                "fingerprint_rsa": "rsa 2048 fingerprint"
            }
        }
    "#;

    let base_paths = PathBase {
        config_dir: "/config".to_string(),
        save_dir: "/var/save".to_string(),
        timeframes_file: None,
        tmp_dir: "/tmp/vbackup".to_string(),
        auth_data_file: None
    };

    let paths = Paths::from(base_paths);
    let modules_paths = paths.for_module("test", "controller", &None, &None);

    let mut controller = modules::controller::get_module("mqtt").unwrap();
    controller.init(&"test", &serde_json::from_str(controller_config).unwrap(), &modules_paths).expect("Failed getting controller");
    let mqtt_result = controller.begin();
        if mqtt_result.is_ok() {
        info!("Controller succeeded, result: {}", mqtt_result.unwrap());
    } else {
        info!("Controller failed to do his thing")
    }
    controller.end().expect("Controller end failed");

    /*let sync = modules::sync::get_module_list().get("duplicati").unwrap().new(&"test", &serde_json::from_str(duplicati_config).unwrap(), &paths, true, false);
    let duplicati_result = sync.unwrap().sync();
    if duplicati_result.is_ok() {
        info!("Duplicati sync succeeded");
    } else {
        info!("Duplicati sync failed");
    }*/

    // util::file::write_if_change("/tmp/foo.txt", Some("600"), "This is the content", true);
}