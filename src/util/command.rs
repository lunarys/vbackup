use crate::{change_error, try_result};

use std::process::{Command, Child, ExitStatus};

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

    pub fn run_or_dry_run(&mut self, dry_run: bool, name: &str) -> Result<(), String> {
        if dry_run {
            info!("DRY-RUN: {}", self.to_string());
        } else {
            let mut process: Child = try_result!(self.spawn(), format!("Failed to start {}", name));
            let exit_status: ExitStatus = try_result!(process.wait(), format!("Failed to run {}", name));

            if !exit_status.success() {
                let msg = format!("Exit code indicates failure of {}", name);
                error!("{}", msg);
                return Err(msg);
            }
        }

        return Ok(());
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