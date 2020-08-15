use crate::processing::preprocessor::{ConfigurationUnit};

pub fn get_exec_order(config_list: Vec<ConfigurationUnit>) -> Result<Vec<ConfigurationUnit>, String> {
    let mut sync_bundles = vec![];
    let mut backup_list = vec![];
    let mut sync_list = vec![];

    for configuration in config_list {
        match configuration {
            ConfigurationUnit::Backup(backup) => {
                backup_list.push(ConfigurationUnit::Backup(backup))
            },
            ConfigurationUnit::Sync(sync) => {
                sync_list.push(ConfigurationUnit::Sync(sync));
            },
            ConfigurationUnit::SyncControllerBundle(sync) => {
                sync_bundles.push(ConfigurationUnit::SyncControllerBundle(sync));
            }
        }
    }

    let mut configuration_list = vec![];

    // First backups
    configuration_list.append(backup_list.as_mut());

    // Then sync bundles
    configuration_list.append(sync_bundles.as_mut());

    // Then syncs
    configuration_list.append( sync_list.as_mut());

    return Ok(configuration_list);
}
