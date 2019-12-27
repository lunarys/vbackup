use crate::{change_error};

use std::process::{Command, Child};

pub struct Paths {
    pub save_path: String,
    pub timeframes_file: String,
    pub tmp_dir: String,
    pub auth_data_file: String,
    pub module_data_dir: String
}

impl Paths {
    pub fn copy(&self) -> Self {
        return Paths {
            save_path: String::from(&self.save_path),
            timeframes_file: String::from(&self.timeframes_file),
            tmp_dir: String::from(&self.tmp_dir),
            auth_data_file: String::from(&self.auth_data_file),
            module_data_dir: String::from(&self.module_data_dir)
        }
    }
}

pub struct CommandWrapper {
    command: Command,
    base: String,
    args: Vec<String>,
    envs: Vec<String>
}

impl CommandWrapper {
    pub fn new(cmd: &str) -> CommandWrapper {
        CommandWrapper {
            command: Command::new(cmd),
            base: cmd.to_string(),
            args: vec![],
            envs: vec![]
        }
    }

    pub fn arg_str(&mut self, arg: &str) -> &mut CommandWrapper {
        self.arg_string(arg.to_string())
    }

    pub fn arg_string(&mut self, option: String) -> &mut CommandWrapper {
        let this = option;
        self.command.arg(&this);
        self.args.push(this);
        self
    }

    pub fn env(&mut self, key: &str, value: &str) {
        self.command.env(key, value);
        self.envs.push(format!("{}={}", key, value));
    }

    pub fn spawn(&mut self) -> Result<Child,String> {
        change_error!(self.command.spawn(), "Failed spawning command")
    }
}

impl ToString for CommandWrapper {
    fn to_string(&self) -> String {
        let mut result = String::new();
        for env in self.envs.iter() {
            result.push_str(env.as_str());
            result.push_str(" ");
        }
        result.push_str(self.base.as_str());
        result.push_str(" ");
        for arg in self.args.iter() {
            result.push_str(arg.as_str());
            result.push_str(" ");
        }
        return result;
    }
}