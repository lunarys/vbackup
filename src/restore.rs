use std::path::Path;
use std::rc::Rc;
use crate::Arguments;
use crate::util::io::json;
use crate::util::objects::configuration::Configuration;
use crate::util::objects::paths::Paths;

pub fn main(mut args: Arguments, paths: Rc<Paths>) -> Result<(),String> {
    let (name, file_path) = if let Some(name) = args.name.as_ref() {
        (name, format!("{}/volumes/{}.json", &paths.config_dir, args.name.as_ref().unwrap()))
    } else {
        return Err(String::from("Please set an volume to restore, batch restore is not supported"));
    };

    info!("Running restore for '{}'", name);

    let config = json::from_file::<Configuration>(Path::new(&file_path))?;

    if let Some(sync_config) = config.sync {
        debug!("Checking sync configuration...");
    } else {
        info!("No sync configuration found...");
    };

    if let Some(backup_config) = config.backup {
        debug!("Checking backup configuration");
    } else {
        info!("No backup configuration found...");
    };

    return Ok(());
}