#[macro_use]
extern crate log;
extern crate env_logger;
extern crate serde_json;
extern crate serde;

extern crate serde_derive;

mod modules;
mod util;

use log::LevelFilter;
use env_logger::Builder;

use crate::modules::traits::Sync;
use crate::modules::traits::Controller;
use crate::modules::object::Paths;

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

    let paths : Paths = Paths {
        save_path: "/save".to_string(),
        timeframes_file: "/timeframes".to_string(),
        tmp_dir: "/tmp_dir".to_string(),
        auth_data_file: "/auth_data".to_string(),
        module_data_dir: "/module_data".to_string()
    };

    let mut controller = modules::controller::get_module("mqtt").unwrap();
    controller.init(&"test", &serde_json::from_str(controller_config).unwrap(), &paths).expect("Failed getting controller");
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