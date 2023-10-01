#[macro_use]
extern crate log;
extern crate env_logger;
extern crate serde_json;
extern crate serde;
extern crate serde_derive;
extern crate chrono;
extern crate glob;
extern crate fs2;

mod processing;
mod vbackup;
mod restore;
mod modules;
mod util;

use log::LevelFilter;
use env_logger::Builder;
use std::fs::OpenOptions;
use fs2::FileExt;
use std::os::unix::fs::OpenOptionsExt;

use std::process::exit;
use argparse::{ArgumentParser, Store, StoreOption, StoreTrue};
use serde::{Deserialize};

#[derive(Deserialize)]
pub struct Arguments {
    pub operation: String,
    pub dry_run: bool,
    pub verbose: bool,
    pub debug: bool,
    pub quiet: bool,
    pub force: bool,
    pub name: Option<String>,
    pub base_config: String,
    pub no_docker: bool,
    pub no_reporting: bool,
    pub override_disabled: bool,
    pub is_restore: bool,
    pub restore_to: Option<String>,
    pub show_command: bool,
    pub show_command_output: bool,
    pub hide_command: bool,
    pub run_manual: bool,
    pub run_all: bool,
    pub run_manual_only: bool,
    pub ignore_time_check: bool,
    pub ignore_additional_check: bool
}

fn main() {
    let mut args = Arguments {
        operation: String::new(),
        dry_run: false,
        verbose: false,
        debug: false,
        quiet: false,
        force: false,
        name: None,
        base_config: String::from("/etc/vbackup/config.json"),
        no_docker: false,
        no_reporting: false,
        override_disabled: false,
        is_restore: false,
        restore_to: None,
        show_command: false,
        show_command_output: false,
        hide_command: false,
        run_manual: false,
        run_all: false,
        run_manual_only: false,
        ignore_time_check: false,
        ignore_additional_check: false
    };

    {
        let mut parser = ArgumentParser::new();
        parser.set_description("Client to interact with a MQTT device controller");
        parser.refer(&mut args.operation)
            .add_argument("operation", Store, "Operation to perform (run,backup,sync,list,version)")
            .required();
        parser.refer(&mut args.name)
            .add_option(&["-n", "--name"], StoreOption, "Name of the specific backup to run");
        parser.refer(&mut args.base_config)
            .add_option(&["-c", "--config"], Store, "Change base configuration file path");
        parser.refer(&mut args.dry_run)
            .add_option(&["--dry-run"], StoreTrue, "Print actions instead of performing them");
        parser.refer(&mut args.verbose)
            .add_option(&["-v", "--verbose", "--trace"], StoreTrue, "Print additional trace information");
        parser.refer(&mut args.debug)
            .add_option(&["-d", "--debug"], StoreTrue, "Print additional debug information");
        parser.refer(&mut args.quiet)
            .add_option(&["-q", "--quiet"], StoreTrue, "Only print warnings and errors");
        parser.refer(&mut args.force)
            .add_option(&["-f", "--force"], StoreTrue, "Force the run to disregard time constraints");
        parser.refer(&mut args.no_docker)
            .add_option(&["-b", "--bare", "--no-docker"], StoreTrue, "'Bare' run without using docker");
        parser.refer(&mut args.no_reporting)
            .add_option(&["--no-reporting"], StoreTrue, "Disable reporting for this run");
        parser.refer(&mut args.override_disabled)
            .add_option(&["--override-disabled", "--run-disabled"], StoreTrue, "Ignore the disabled status on configurations");
        parser.refer(&mut args.restore_to)
            .add_option(&["--restore-to"], StoreOption, "Restore only: Restore to the given directory");
        parser.refer(&mut args.show_command)
            .add_option(&["--show-command", "--print-command"], StoreTrue, "Print the commands that are executed. Default for debug and verbose log level");
        parser.refer(&mut args.show_command_output)
            .add_option(&["-o", "--show-command-output"], StoreTrue, "Do not print command output when printing executed commands");
        parser.refer(&mut args.hide_command)
            .add_option(&["--hide-command"], StoreTrue, "Disable default command output for verbose or debug logging");
        parser.refer(&mut args.run_manual)
            .add_option(&["--manual"], StoreTrue, "Run only configurations that are marked as manual");
        parser.refer(&mut args.run_all)
            .add_option(&["--all"], StoreTrue, "Run everything except disabled configurations, includes run manual configurations");
        parser.refer(&mut args.ignore_time_check)
            .add_option(&["--ignore-time-check", "--ignore-time-checks"], StoreTrue, "Disable all time checks");
        parser.refer(&mut args.ignore_additional_check)
            .add_option(&["--ignore-additional-check", "--ignore-additional-checks"],StoreTrue, "Disable all additional checks");
        parser.parse_args_or_exit();
    }

    // all implies manual, but manual implies not all
    if args.run_all {
        args.run_manual = true;
        args.run_manual_only = false;
    } else if args.run_manual {
        args.run_manual_only = true;
    }

    let log_level = if args.verbose {
        LevelFilter::Trace
    } else if args.debug {
        LevelFilter::Debug
    } else if args.quiet {
        LevelFilter::Warn
    } else {
        LevelFilter::Info
    };

    // set defaults for command printing
    if !args.hide_command {
        args.show_command_output |= args.verbose | args.debug;
    }
    args.show_command |= args.show_command_output;

    // TODO: Always prints timestamps in UTC
    Builder::new()
        .filter_level(LevelFilter::Warn)
        .filter_module("vbackup", log_level)
        .init();

    run(args);
}

fn run(args: Arguments) {
    let operation = args.operation.clone();
    let version = env!("CARGO_PKG_VERSION");

    if operation == "version" {
        println!("vbackup v{}", version);
        exit(0);
    }

    info!("Starting '{}' (v{})", operation.as_str(), version);

    // Ensure only one instance of this executable is running
    let lock_file_result = OpenOptions::new()
        .create(true) // Create file if it does not exist
        .read(true)
        .write(true)
        .mode(u32::from_str_radix("600", 8).unwrap()) // Only sets mode when creating the file...
        .open("/run/vbackup.lock");
    if let Err(err) = lock_file_result {
        error!("Could not access lock file for vbackup: {}", err.to_string());
        exit(1);
    }

    let lock_file = lock_file_result.unwrap();

    if lock_file.try_lock_exclusive().is_err() {
        error!("Could not acquire file lock for vbackup, is it already running?");
        exit(2);
    }

    let result = vbackup::main(args);
    if let Err(error) = result.as_ref() {
        error!("vbackup run failed: {}", error);
    }

    if lock_file.unlock().is_err() {
        error!("Releasing file lock failed");
        exit(4);
    }

    if result.is_err() {
        exit(3);
    }

    info!("Done with '{}'", operation.as_str());
}