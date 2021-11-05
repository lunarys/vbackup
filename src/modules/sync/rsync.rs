use crate::modules::traits::Sync;
use crate::util::command::CommandWrapper;
use crate::util::io::{file,json,auth_data};
use crate::util::docker;
use crate::util::objects::paths::{ModulePaths,SourcePath};
use crate::util::objects::shared::ssh::{SshConfig, write_identity_file, write_known_hosts};
use crate::Arguments;

use serde_json::Value;
use serde::{Deserialize};

pub struct Rsync {
    _name: String,
    config: Configuration,
    ssh_config: SshConfig,
    module_paths: ModulePaths,
    sync_paths: DockerPaths,
    dry_run: bool,
    no_docker: bool,
    verbose: bool,
    print_command: bool
}

struct DockerPaths {
    volume: Option<SourcePath>,
    from: String,
    to: String
}

#[derive(Deserialize)]
struct Configuration {
    #[serde(default="default_true")]
    to_remote: bool,
    #[serde(default="default_false")]
    compress: bool,
    path_prefix: Option<String>,
    dirname: String,

    #[serde(default="default_chmod_perms")]
    chmod_perms: String,
    // Add additional options for local and remote, to apply to the respective sync direction
    local_chmod: Option<String>,
    remote_chmod: Option<String>,

    local_chown: Option<String>,

    filter: Option<Vec<String>>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,

    // Configuration option for rsync executable
    #[serde(default="default_rsync")]
    local_rsync: String,
    remote_rsync: Option<String>,

    host: Option<Value>,
    host_reference: Option<String>,

    // Option to inject additional arguments
    additional_args: Option<Vec<String>>,

    // detect-renamed activated would be the better options, but only works with patched servers
    //  so set it disabled by default
    #[serde(default="default_false")]
    detect_renamed: bool,
    #[serde(default="default_false")]
    detect_renamed_lax: bool,
    #[serde(default="default_false")]
    detect_moved: bool
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_chmod_perms() -> String { String::from("D0775,F0664") }
fn default_rsync() -> String { String::from("rsync") }

impl Sync for Rsync {
    const MODULE_NAME: &'static str = "rsync-ssh";

    fn new(name: &str, config_json: &Value, module_paths: ModulePaths, args: &Arguments) -> Result<Box<Self>, String> {
        let config = json::from_value::<Configuration>(config_json.clone())?; // TODO: - clone
        let ssh_config = auth_data::resolve::<SshConfig>(&config.host_reference, &config.host, module_paths.base_paths.as_ref())?;

        let remote_path = format!("{}@{}:{}",
                                  ssh_config.user,
                                  ssh_config.hostname,
                                  config.path_prefix.as_ref().map_or("", |prefix| prefix.as_str()));

        let separator = if let Some(prefix) = config.path_prefix.as_ref() {
            if prefix.eq("") || prefix.eq("/") {
                // Putting a separating slash after an empty string or slash only makes no sense
                ""
            } else {
                "/"
            }
        } else {
            ""
        };

        let paths = if args.no_docker {
            if let SourcePath::Single(source_path) = module_paths.source.clone() {
                if config.to_remote {
                    DockerPaths {
                        volume: None,
                        from: source_path,
                        to: remote_path,
                    }
                } else {
                    DockerPaths {
                        volume: None,
                        from: format!("{}{}{}", remote_path, separator, config.dirname),
                        to: source_path,
                    }
                }
            } else {
                return Err(String::from("Multiple source paths are not supported in rsync module without docker"));
            }
        } else {
            if config.to_remote {
                DockerPaths {
                    volume: Some(module_paths.source.clone()),
                    from: format!("/{}", config.dirname),
                    to: remote_path
                }
            } else {
                DockerPaths {
                    volume: Some(module_paths.source.clone()),
                    from: format!("{}{}{}", remote_path, separator, config.dirname),
                    to: String::from("/")
                }
            }
        };

        return Ok(Box::new(Self {
            _name: String::from(name),
            config,
            ssh_config,
            module_paths,
            sync_paths: paths,
            dry_run: args.dry_run,
            no_docker: args.no_docker,
            verbose: args.verbose,
            print_command: args.debug || args.verbose
        }));
    }

    fn init(&mut self) -> Result<(), String> {
        // Build local docker image if missing
        if !self.no_docker {
            if self.config.detect_renamed || self.config.detect_renamed_lax || self.config.detect_moved {
                docker::build_image_if_missing(&self.module_paths.base_paths, "rsync-patched.Dockerfile", "vbackup-rsync-patched")?;
            } else {
                docker::build_image_if_missing(&self.module_paths.base_paths, "rsync.Dockerfile", "vbackup-rsync")?;
            }
        }

        write_known_hosts(&self.ssh_config, &self.module_paths, self.dry_run)?;
        write_identity_file(&self.ssh_config, &self.module_paths, self.dry_run)?;

        return Ok(());
    }

    fn sync(&self) -> Result<(), String> {
        let mut command = self.get_base_cmd()?;

        if self.config.detect_renamed {
            command.arg_str("--detect-renamed");
        }

        if self.config.detect_renamed_lax {
            command.arg_str("--detect-renamed-lax");
        }

        if self.config.detect_moved {
            command.arg_str("--detect-moved");
        }

        command.arg_string(format!("{}", &self.sync_paths.from))
            .arg_string(format!("{}", &self.sync_paths.to));

        command.run_configuration(self.print_command, self.dry_run)?;

        return Ok(());
    }

    fn restore(&self) -> Result<(), String> {
        let mut command = self.get_base_cmd()?;

        command.arg_string(format!("{}", &self.sync_paths.to))
            .arg_string(format!("{}", &self.sync_paths.from));

        command.run_configuration(self.print_command, self.dry_run)?;

        return Ok(());
    }

    fn clear(&mut self) -> Result<(), String> {
        return Ok(());
    }
}

impl Rsync {
    fn get_base_cmd(&self) -> Result<CommandWrapper,String> {
        if !self.dry_run {
            file::create_dir_if_missing(self.module_paths.module_data_dir.as_str(), true)?;
        }

        // Distinguish run in docker and directly on the machine
        let mut command = if let Some(docker_paths) = self.sync_paths.volume.as_ref() {
            // End docker command with docker image name
            let image_name = if self.config.detect_renamed || self.config.detect_renamed_lax || self.config.detect_moved {
                "vbackup-rsync-patched"
            } else {
                "vbackup-rsync"
            };

            CommandWrapper::new_docker(
                "rsync-vbackup-tmp",
                image_name,
                Some(self.config.local_rsync.as_str()),
                &self.module_paths,
                (docker_paths, &self.config.dirname),
                Some(vec![
                    "--env=SSHPASS"
                ])
            )
        } else {
            CommandWrapper::new(self.config.local_rsync.as_str())
        };

        // Authentication: password or private key
        command.arg_str("-e");

        // set up the ssh command
        command.append_ssh_command(&self.ssh_config, &self.module_paths, !self.no_docker, false)?;

        // Default sync options
        command.arg_str("--archive")
            .arg_str("--delete")
            .arg_str("--partial");

        // Set file permissions for receiving end
        command.arg_str("--perms");
        if (self.config.to_remote
                && self.config.remote_chmod.is_none())
            || (!self.config.to_remote
                && self.config.local_chmod.is_none()) {

            command.arg_string(format!("--chmod={}", self.config.chmod_perms.as_str()));
        } else {
            if self.config.to_remote {
                command.arg_string(format!("--chmod={}", self.config.remote_chmod.as_ref().unwrap()));
            } else {
                command.arg_string(format!("--chmod={}", self.config.local_chmod.as_ref().unwrap()));
            }
        }

        // If copying to the local filesystem, set owning user and group
        if !self.config.to_remote {
            if let Some(chown_string) = self.config.local_chown.as_ref() {
                command.arg_string(format!("--chown={}", chown_string));
            }
        }

        // Parse include and exclude options
        if self.config.filter.is_some() || self.config.include.is_some() || self.config.exclude.is_some() {
            if let Some(filter_list) = self.config.filter.as_ref() {
                filter_list.iter().for_each(|filter_option| {
                    command.arg_string(format!("--filter={}", filter_option));
                });
            }

            if let Some(exclude_list) = self.config.exclude.as_ref() {
                exclude_list.iter().for_each(|exclude_path| {
                    command.arg_string(format!("--exclude={}", exclude_path));
                });
            }

            if let Some(include_list) = self.config.include.as_ref() {
                // Include only works with including directories by default, so remove empty ones
                command.arg_str("--prune-empty-dirs");
                command.arg_str("--include=*/");

                include_list.iter().for_each(|include_path| {
                    command.arg_string(format!("--include={}", include_path));
                });

                // Argument order matters, so exclude everything else last
                command.arg_str("--exclude=*");
            }
        }

        if let Some(rsync_path) = self.config.remote_rsync.as_ref() {
            command.arg_string(format!("--rsync-path={}", rsync_path));
        }

        if self.config.compress {
            command.arg_str("--compress");
        }

        if self.dry_run {
            command.arg_str("--dry-run");
        }

        if self.verbose {
            command.arg_str("--verbose");
        }

        if let Some(args) = self.config.additional_args.as_ref() {
            for arg in args {
                command.arg_str(arg.as_str());
            }
        }

        return Ok(command);
    }
}