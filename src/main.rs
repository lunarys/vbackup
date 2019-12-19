#[macro_use]
extern crate log;
extern crate env_logger;
extern crate serde_json;
extern crate serde;

mod module_traits;
mod modules;

use log::LevelFilter;
use env_logger::Builder;

use modules::traits::Sync;
use modules::traits::Controller;

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
            "port": 1883
       }
    "#;

    modules::controller::get_module_list().get("mqtt").unwrap().begin("test".to_string(), serde_json::from_str(data).unwrap());
}