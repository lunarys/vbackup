#[macro_use]
extern crate log;
extern crate env_logger;
extern crate serde_json;
extern crate serde;
extern crate serde_derive;
extern crate chrono;
extern crate glob;
extern crate fs2;

mod vbackup;
mod modules;
mod util;

use log::LevelFilter;
use env_logger::Builder;
use std::io::Result;
use std::fs::{File, OpenOptions};
use fs2::FileExt;
use std::os::unix::fs::OpenOptionsExt;

use crate::modules::traits::Sync;
use crate::modules::traits::Controller;
use crate::modules::object::{Paths, PathBase, Arguments};
use std::process::exit;
use argparse::{ArgumentParser, Store, StoreOption, StoreTrue};

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
        no_docker: false
    };

    {
        let mut parser = ArgumentParser::new();
        parser.set_description("Client to interact with a MQTT device controller");
        parser.refer(&mut args.operation)
            .add_argument("operation", Store, "Operation to perform (run,backup,sync,list)")
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
        parser.parse_args_or_exit();
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

    Builder::new()
        .filter_level(log_level)
        .filter_module("paho_mqtt", LevelFilter::Warn)
        .init();

    run(args);
}

fn run(args: Arguments) {
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
}