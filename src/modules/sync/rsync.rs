use crate::modules::traits::Sync;
use crate::modules::object::{Paths, CommandWrapper};
use crate::util::auth_data;

use crate::{try_result,try_option,auth_resolve,conf_resolve};

use serde_json::Value;
use serde::{Deserialize};
use std::process::{Child, ExitStatus};

pub struct Rsync {
    bind: Option<Bind>
}

struct Bind {

}

#[derive(Deserialize)]
struct Configuration {
    #[serde(default="default_to_remote")]
    to_remote: bool,
    #[serde(default="default_compress")]
    compress: bool,
    dirname: String,

    auth: Option<Value>,
    auth_reference: Option<String>
}

fn default_to_remote() -> bool { true }
fn default_compress() -> bool { false }

#[derive(Deserialize)]
struct Authentication {
    hostname: String,
    port: i32,
    user: String,
    password: Option<String>,
    login_key: Option<String>, // SSH private key (unencrypted)
    host_key: String // SSH public key of host
}

impl Rsync {
    pub fn new_empty() -> Self {
        return Rsync { bind: None }
    }
}

impl Sync for Rsync {
    fn init(&mut self, name: &str, config_json: &Value, paths: &Paths, dry_run: bool, no_docker: bool) -> Result<(), String> {
        unimplemented!()
    }

    fn sync(&self) -> Result<(), String> {
        unimplemented!()
    }

    fn restore(&self) -> Result<(), String> {
        unimplemented!()
    }

    fn clear(&mut self) -> Result<(), String> {
        unimplemented!()
    }
}