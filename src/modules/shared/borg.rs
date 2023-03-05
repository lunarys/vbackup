use serde::{Deserialize};
use serde_json::Value;
use crate::util::io::{file, json};
use crate::util::objects::paths::{ModulePaths, SourcePath};
use crate::Arguments;
use crate::util::command::CommandWrapper;
use std::borrow::Borrow;
use std::ops::Sub;
use std::rc::Rc;
use crate::modules::sync::borg::BorgSyncConfig;
use crate::util::docker;
use crate::modules::shared::ssh::{write_known_hosts, write_identity_file};
use crate::util::io::user::{ask_user_option_list_index};

#[derive(Deserialize)]
struct BorgKeepConfig {
    within: Option<String>,
    secondly: Option<u8>,
    minutely: Option<u8>,
    hourly: Option<u8>,
    daily: Option<u8>,
    weekly: Option<u8>,
    monthly: Option<u8>,
    yearly: Option<u8>
}

#[derive(Deserialize)]
struct BorgConfig {
    encryption_key: Option<String>,
    authentication_key: Option<String>,
    #[serde(default="default_true")]
    blake2: bool,
    quota: Option<String>,

    #[serde(default="default_false")]
    no_init: bool,
    #[serde(default="default_false")]
    append_only: bool,
    _keyfile: Option<String>,

    prefix: Option<String>,
    exclude: Option<Vec<String>>,
    additional_options: Option<Vec<String>>,

    keep: BorgKeepConfig,
    #[serde(default="default_false")]
    disable_prune: bool,
    #[serde(default="default_false")]
    relocate_ok: bool,

    #[serde(default="default_umask")]
    umask: String
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_umask() -> String { String::from("0007") }

pub struct Borg {
    config: BorgConfig,
    sync_config: Option<BorgSyncConfig>,
    paths: ModulePaths,
    dry_run: bool,
    no_docker: bool,
    print_command: bool,
    requires_init: bool,
    verbose: bool,
    restore_to: Option<String>,
    is_restore: bool
}

impl Borg {
    pub fn new(_name: &str, config_json: &Value, paths: ModulePaths, args: &Rc<Arguments>, sync_config: Option<BorgSyncConfig>) -> Result<Box<Self>, String> {
        let config = json::from_value::<BorgConfig>(config_json.clone())?; // TODO: - clone

        if args.no_docker && args.is_restore && args.restore_to.is_some() {
            return Err(String::from("The restore-to option is not supported for borg without docker"));
        }

        return Ok(Box::new(Self {
            config,
            sync_config,
            paths,
            dry_run: args.dry_run,
            verbose: args.verbose,
            no_docker: args.no_docker,
            print_command: args.verbose || args.debug,
            restore_to: args.restore_to.clone(),
            is_restore: args.is_restore,
            requires_init: false // initial value overwritten in init step
        }));
    }

    pub fn init(&mut self) -> Result<(), String> {

        // create the module data directory if it does not exist
        file::create_dir_if_missing(self.paths.module_data_dir.as_str(), true)?;

        // Create a marker file to determine whether the repo has been initialized
        //  run repo init only in save later, as it possibly involves a ssh connection and file creation
        if !self.config.no_init && !file::exists(format!("{}/init-marker", self.paths.module_data_dir).as_str()) {
            self.requires_init = true;
        }

        if !self.no_docker {
            docker::build_image_if_missing(&self.paths.base_paths, "borg.Dockerfile", "vbackup-borg")?;
        }

        if self.dry_run {
            return Ok(());
        }

        if let Some(sync_config) = self.sync_config.as_ref() {
            write_known_hosts(sync_config.ssh_config.borrow(), &self.paths, self.dry_run)?;
            write_identity_file(sync_config.ssh_config.borrow(), &self.paths, self.dry_run)?;
        }

        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), String> {

        // nothing to do
        Ok(())
    }

    fn run_init(&self) -> Result<(), String> {
        let mut command = self.get_base_cmd("init")?;

        command.arg_str("--encryption");

        if self.config.encryption_key.is_some() {
            if self.config.blake2 {
                command.arg_str("repokey-blake2");
            } else {
                command.arg_str("repokey");
            }
        } else if self.config.authentication_key.is_some() {
            if self.config.blake2 {
                command.arg_str("authenticated-blake2");
            } else {
                command.arg_str("authenticated");
            }
        } else {
            command.arg_str("none");
        }

        if let Some(quota) = self.config.quota.as_ref() {
            command.arg_string(format!("--storage-quota={}", quota));
        }

        if self.config.append_only {
            command.arg_str("--append-only");
        }

        command.arg_str("--make-parent-dirs");
        command.arg_string(self.get_repo_path());

        command.run_configuration(self.print_command, self.dry_run)
    }

    fn run_prune(&self) -> Result<(), String> {
        let mut command = self.get_base_cmd("prune")?;

        /*
         * limit to proper prefix
         */
        if let Some(prefix) = self.config.prefix.as_ref() {
            command.arg_string(format!("--prefix=vbackup_{}_", prefix));
        } else {
            command.arg_str("--prefix=vbackup_");
        }

        /*
         * add options
         */
        if self.dry_run {
            command.arg_str("--dry-run");
        }

        if self.verbose {
            command.arg_str("--stats");
            command.arg_str("--list");
        }

        if let Some(within) = self.config.keep.within.as_ref() {
            command.arg_string(format!("--keep-within={}", within));
        }

        if let Some(secondly) = self.config.keep.secondly {
            command.arg_string(format!("--keep-secondly={}", secondly));
        }

        if let Some(minutely) = self.config.keep.minutely {
            command.arg_string(format!("--keep-minutely={}", minutely));
        }

        if let Some(hourly) = self.config.keep.hourly {
            command.arg_string(format!("--keep-hourly={}", hourly));
        }

        if let Some(daily) = self.config.keep.daily {
            command.arg_string(format!("--keep-daily={}", daily));
        }

        if let Some(weekly) = self.config.keep.weekly {
            command.arg_string(format!("--keep-weekly={}", weekly));
        }

        if let Some(monthly) = self.config.keep.monthly {
            command.arg_string(format!("--keep-monthly={}", monthly));
        }

        if let Some(yearly) = self.config.keep.yearly {
            command.arg_string(format!("--keep-yearly={}", yearly));
        }

        command.arg_string(self.get_repo_path());

        command.run_configuration(self.print_command, self.dry_run)
    }

    pub fn run_save(&self) -> Result<(), String> {
        /*
         * Init repository if necessary
         */
        if self.requires_init {
            self.run_init()?;

            if !self.dry_run {
                file::write(format!("{}/init-marker", self.paths.module_data_dir).as_str(), "1", true)?;
            }
        }

        /*
         * Start backup command
         */
        let mut command = self.get_base_cmd("create")?;

        /*
         * add options
         */
        if self.dry_run {
            command.arg_str("--dry-run");
        }

        if self.verbose {
            command.arg_str("--stats");
            command.arg_str("--list");
        }

        if let Some(excludes) = self.config.exclude.as_ref() {
            for exclude in excludes {
                command.arg_string(format!("--exclude={}", exclude));
            }
        }

        if let Some(additional_options) = self.config.additional_options.as_ref() {
            for arg in additional_options {
                command.arg_str(arg);
            }
        }

        /*
         * append repo location
         */
        let prefix = if let Some(prefix) = self.config.prefix.as_ref() {
            format!("vbackup_{}", prefix)
        } else {
            String::from("vbackup")
        };
        command.arg_string(format!("{}::{}_{}",
                                   self.get_repo_path(),
                                   prefix,
                                   "{now:%Y-%m-%dT%H:%M:%S}"
        ));

        /*
         * append paths to save
         */
        match self.paths.source.borrow() {
            SourcePath::Single(path) => {
                if self.no_docker {
                    command.arg_str(path);
                } else {
                    command.arg_str("/volume");
                }
            }
            SourcePath::Multiple(paths) => {
                for path in paths {
                    if self.no_docker {
                        command.arg_str(path.path.as_str());
                    } else {
                        command.arg_string(format!("/volume/{}", path.name));
                    }
                }
            }
        }

        command.run_configuration(self.print_command, self.dry_run)?;

        if !self.config.disable_prune {
            self.run_prune()?;
        } else {
            debug!("Pruning the borg repository is disabled");
        }

        Ok(())
    }

    pub fn run_restore(&self) -> Result<(), String> {
        // if the repo is not initialized during a restore operation assume it was recovered from some other location and does not need to be initialized
        if self.requires_init && !self.dry_run {
            file::write(format!("{}/init-marker", self.paths.module_data_dir).as_str(), "1", true)?;
        }

        // TODO: borg writes into the current directory ("."), so better make sure to use "cd /"

        let selected_archive = self.user_select_archive()?;
        let mut command = self.get_base_cmd("extract")?;

        if self.dry_run {
            command.arg_str("--dry-run");
        }

        // TODO: does this make sense?
        //command.arg_str("--numeric-ids");

        if self.verbose {
            command.arg_str("--list");
        }

        // archive path is constructed from repo and archive
        command.arg_string(format!("{}::{}", self.get_repo_path(), selected_archive));

        info!("Starting restore of '{}'...", selected_archive);
        command.run_configuration(self.print_command, self.dry_run)?;
        info!("Restore done.");

        Ok(())
    }

    fn user_select_archive(&self) -> Result<String, String> {
        let mut archive_list = self.get_list_of_archives(Some("{archive}{NEWLINE}"))?; // would also work with '--short' option

        // the list is sorted by timestamp (borg default) so suggest the last entry
        let selected_index = ask_user_option_list_index(
            Some("Found multiple backup archives in the borg repository:"),
            Some("Which archive should be restored?"),
            archive_list.as_ref(),
            &|archive| {archive.as_str()},
            archive_list.len().sub(1)
        )?;

        return if selected_index < archive_list.len() {
            Ok(archive_list.swap_remove(selected_index))
        } else {
            Err(String::from("Selected index is out of bounds"))
        }
    }

    /*
     * format according to borg list's --format option
     * entries should be newline-separated if passing a custom format
     */
    fn get_list_of_archives(&self, format: Option<&str>) -> Result<Vec<String>, String> {
        let mut command = self.get_base_cmd("list")?;

        if let Some(format) = format {
            command.arg_string(format!("--format={}", format));
        }

        command.arg_string(self.get_repo_path());

        return command.run_get_output().map(|result| {
            result.split("\n").map(String::from).collect()
        });
    }

    fn _run_check_backup(&self) -> Result<(), String> {
        todo!("check the consistency of the backup?")
    }

    fn get_base_cmd(&self, operation: &str) -> Result<CommandWrapper,String> {
        let mut command;
        if self.no_docker {
            command = CommandWrapper::new_with_args("borg", vec![operation]);

            command.env("BORG_BASE_DIR", self.paths.module_data_dir.as_str());
            command.env("BORG_RELOCATED_REPO_ACCESS_IS_OK", if self.config.relocate_ok {"yes"} else {"no"})
        } else {
            let mut options = vec![
                "--env=BORG_BASE_DIR",
                "--env=BORG_PASSPHRASE",
                "--env=SSHPASS"
            ];

            if self.config.relocate_ok {
                options.push("--env=BORG_RELOCATED_REPO_ACCESS_IS_OK=yes");
            } else {
                options.push("--env=BORG_RELOCATED_REPO_ACCESS_IS_OK=no");
            }

            let volume_mount_arg;
            if self.sync_config.as_ref().is_none() {
                volume_mount_arg = format!("--volume={}:/destination", self.paths.destination);
                options.push(volume_mount_arg.as_str());
            }

            let mut source_overwrite = None;
            if self.is_restore {
                if let Some(restore_to) = self.restore_to.as_ref() {
                    source_overwrite.replace(SourcePath::Single(restore_to.clone()));
                }
            }

            let source_mount =  source_overwrite.as_ref().unwrap_or(self.paths.source.borrow());

            command = CommandWrapper::new_docker(
                "borg-vbackup-tmp",
                "vbackup-borg",
                Some("borg"),
                Some(vec![operation]),
                &self.paths,
                (source_mount, "/volume"),
                Some(options)
            );

            command.env("BORG_BASE_DIR", "/module");
        };

        if let Some(passphrase) = self.config.encryption_key.as_ref().or(self.config.authentication_key.as_ref()) {
            command.env("BORG_PASSPHRASE", passphrase);
        }

        // configure ssh connection
        if let Some(borg_sync) = self.sync_config.as_ref() {
            let ssh_command = command.build_ssh_command(&borg_sync.ssh_config, &self.paths, !self.no_docker, false);
            command.arg_string(format!("--rsh={}", ssh_command));
        }

        // set umask
        command.arg_string(format!("--umask={}", self.config.umask));

        /*if self.verbose {
            command.arg_str("--debug");
        }*/

        return Ok(command);
    }

    fn get_repo_path(&self) -> String {
        if let Some(borg_sync) = self.sync_config.as_ref() {
            let path = if borg_sync.directory.starts_with("/") {
                borg_sync.directory.clone()
            } else if borg_sync.directory.starts_with("~") {
                format!("/{}", borg_sync.directory)
            } else {
                format!("/./{}", borg_sync.directory)
            };

            format!("ssh://{}@{}:{}{}", borg_sync.ssh_config.user, borg_sync.ssh_config.hostname, borg_sync.ssh_config.port, path)
        } else {
            if self.no_docker {
                self.paths.destination.clone()
            } else {
                String::from("/destination")
            }
        }
    }
}