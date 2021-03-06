use crate::util::objects::paths::SourcePath;
use crate::{change_error, try_result, dry_run};

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

    pub fn add_docker_volume_mapping(&mut self, source_path: &SourcePath, name: &str) -> &mut CommandWrapper {
        match source_path {
            SourcePath::Single(path) => {
                self.arg_string(format!("--volume={}:/{}", path, name));
            },
            SourcePath::Multiple(paths) => {
                for path in paths {
                    self.arg_string(format!("--volume={}:/{}/{}", path.path, name, path.name));
                }
            }
        }

        return self;
    }

    pub fn env(&mut self, key: &str, value: &str) {
        self.command.env(key, value);
        //self.envs.push(format!("{}={}", key, value));
        self.envs.push(format!("{}=xxx", key));
    }

    pub fn spawn(&mut self) -> Result<Child,String> {
        change_error!(self.command.spawn(), "Failed spawning command")
    }

    pub fn run(&mut self) -> Result<(), String> {
        let exit_status = self.run_get_status()?;
        if !exit_status.success() {
            let msg = format!("Exit code indicates failure of command");
            error!("{}", msg);
            return Err(msg);
        }
        return Ok(());
    }

    pub fn run_configuration(&mut self, print: bool, dry_run: bool) -> Result<(),String> {
        if dry_run {
            dry_run!(self.to_string());
            return Ok(());
        } else if print {
            println!("-> {}", self.to_string());
            return self.run();
        } else {
            return self.run_without_output();
        }
    }

    pub fn run_without_output(&mut self) -> Result<(), String> {
        let exit_status = self.run_get_status_without_output()?;
        if !exit_status.success() {
            let msg = format!("Exit code indicates failure of command");
            error!("{}", msg);
            return Err(msg);
        }
        return Ok(());
    }

    pub fn _run_or_dry_run(&mut self, dry_run: bool) -> Result<(), String> {
        if dry_run {
            dry_run!(self.to_string());
        } else {
            return self.run();
        }

        return Ok(());
    }

    pub fn _run_or_dry_run_without_output(&mut self, dry_run: bool) -> Result<(), String> {
        if dry_run {
            dry_run!(self.to_string());
        } else {
            return self.run_without_output();
        }

        return Ok(());
    }

    pub fn run_get_status_without_output(&mut self) -> Result<ExitStatus, String> {
        let result = self.command.output();

        if let Ok(output) = result {
            return Ok(output.status);
        } else {
            return Err(format!("Failed executing command: {}", result.unwrap_err().to_string()))
        }
    }

    pub fn run_get_status(&mut self) -> Result<ExitStatus, String> {
        let mut process: Child = try_result!(self.spawn(), "Failed to start command execution");
        let exit_status: ExitStatus = try_result!(process.wait(), "Failed to run command");
        return Ok(exit_status);
    }

    pub fn run_get_output(&mut self) -> Result<String,String> {
        let result = self.command.output();

        if let Ok(output) = result {
            if output.status.success() {
                if let Some((_, output_without_newline)) = output.stdout.split_last() {
                    let output_str: String = try_result!(String::from_utf8(output_without_newline.to_vec()), "Command output can't be converted from UTF-8");
                    return Ok(output_str);
                } else {
                    return Ok(String::new());
                }
            } else {
                return Err(String::from("Exit code indicates failure of command"));
            }
        } else {
            return Err(format!("Failed executing command: {}", result.unwrap_err().to_string()))
        }
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