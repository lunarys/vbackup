#[macro_use]
extern crate log;
extern crate env_logger;
extern crate serde_json;
extern crate serde;

mod modules;
mod util;

use log::LevelFilter;
use env_logger::Builder;

use crate::modules::traits::Sync;
use crate::modules::traits::Controller;
use crate::modules::object::Paths;

fn main() {
    Builder::new()
        .filter_level(LevelFilter::Info)
        .filter_module("paho_mqtt", LevelFilter::Warn)
        .init();

    info!("Hello, world!");

    //modules::sync::get_module_list().get("duplicati").unwrap().sync();

    let data = r#"
       {
            "start": true,
            "device": "sundavar",
            "user": "ju",
            "password": "testpass",
            "host": "ebrithil.elda",
            "port": 1883
       }
    "#;

    let paths : Paths = Paths {
        save_path: "/save".to_string(),
        timeframes_file: "/timeframes".to_string(),
        tmp_dir: "/tmp_dir".to_string(),
        auth_data_file: "/auth_data".to_string(),
        module_data_dir: "/module_data".to_string()
    };

    modules::controller::get_module_list().get("mqtt").unwrap().begin("test".to_string(), &serde_json::from_str(data).unwrap(), &paths)
        .expect("Module mqtt failed!");
}