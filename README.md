# vbackup
## What is this?
This project is a backup solution specifically suited for my needs and setup of my devices. 
It is basically a wrapper for different backup and synchronization solutions, 
that runs those if all configured conditions are met. 
Since I do not keep devices running all the time, 
controllers for remote devices are included, 
ensuring that required devices are actually online for syncing. 
The whole project started as a simple Bash script creating a backup of docker volumes as archives, 
hence the name vbackup.

I started learning Rust with this project, 
so there are some imperfections that I either learned about later or am still learning about. 
Some parts are also not written as general as I'd like them to, 
but this is a tradeoff I was willing to make considering the effort.

## How does it work?
In general backups for directories or docker volumes are defined in a new configuration file.
A backup may consist of a **backup** and a **sync** part or either one of those.
Backups generally only create a *local* backup of the source, while syncs transfer the source 
to a remote destination. If both are defined, a local backup is created and then 
transferred to the remote device instead of directly syncing the source,
but only if a backup that has not been synced before exists.  

## Related projects
- [Backup trigger](https://github.com/lunarys/mqtt-vbackup-trigger): Remotely start the backup over MQTT
- [Device controller](https://github.com/lunarys/mqtt-device-controller): Remotely start and stop devices for the backup over MQTT  
- [Shutdown trigger](https://github.com/lunarys/mqtt-shutdown-trigger): Shut down a device over MQTT

## Running as a service
I use MQTT to trigger runs. 
For that I wrote a simple service running the executable whenever a specific message is received on specific MQTT topics. 
This ensures the synchronization on different devices is run at the same time, 
such that the backup server is not started for each sync separately and can profit from multiple syncs while it is online.

## Command line arguments
`vbackup <operation> [options]`

| Operation | Description             |
|-----------|-------------------------|
| run       | Run backup & sync       |
| backup    | Run only backup         |
| sync      | Run only sync           |
| list      | List all configurations |

| Option                              | is flag |      Default value       | Description                                                                                                                                   |
|-------------------------------------|---------|:------------------------:|-----------------------------------------------------------------------------------------------------------------------------------------------|
| -n, --name                          | no      |                          | Name of a specific configuration to run operation on.                                                                                         |
| -c, --config                        | no      | /etc/vbackup/config.json | Specify the base configuration file.                                                                                                          |
| --dry-run                           | yes     |          false           | Do not perform any permanent changes, instead print what would be done.                                                                       |
| -v, --verbose                       | yes     |          false           | Enable verbose logging (Loglevel: Trace).                                                                                                     |
| -d, --debug                         | yes     |          false           | Enable debug logging (Loglevel: Debug).                                                                                                       |
| -q, --quiet                         | yes     |          false           | Disable info logging (Loglevel: Warn).                                                                                                        |
| -f, --force                         | yes     |          false           | Disregard all constraints, forcing the run.                                                                                                   |
| -b, --bare, --no-docker             | yes     |          false           | Do not use docker. Warning: Can't backup docker volumes (duh!) and might affect the structure of the resulting backup. Not tested thoroughly. |
| --no-reporting                      | yes     |          false           | Disable reporting for this run.                                                                                                               |
| --override-disabled, --run-disabled | yes     |          false           | Ignore the disabled status on configurations.                                                                                                 |

## Requirements
### Docker mode
- docker
### No docker mode
- rsync (for rsync module)
- sshpass (for rsync with password)
- duplicati (for duplicati module)
- 7z / p7zip-full on debian (for tar7zip backup)

Note: this list is probably incomplete.

## Configuration
### Base configuration
Default file: `/etc/vbackup/config.json`

| Key               | Required | Default                   | Description                                                                                                  |
|-------------------|----------|---------------------------|--------------------------------------------------------------------------------------------------------------|
| config_dir        | no       | /etc/vbackup              | Defines the base directory for all configuration files.                                                      |
| save_dir          | no       | /var/vbackup              | Defines the base directory for all saves.                                                                    |
| tmp_dir           | no       | /tmp/vbackup              | Defines the base directory for temporary files.                                                              |
| timeframes_file   | no       | $base_dir/timeframes.json | Path to the file containing all timeframe definitions.                                                       |
| auth_data_file    | no       | $base_dir/auth_data.json  | Path to the file containing all shared authentication data.                                                  |
| reporting_file    | no       | $base_dir/reporting.json  | Path to the file containing all reporting module configurations.                                             |
| docker_images     | no       | $base_dir/images          | Path to the directory containing all docker files.                                                           |
| savedata_in_store | no       | false                     | Flag for writing all savedata into the store_path of the configuration instead of the module data directory. |

```json
{
  "config_dir": "/etc/vbackup",
  "save_dir": "/var/vbackup",
  "timeframes_file": "/etc/vbackup/timeframes.json",
  "tmp_dir": "/tmp/vbackup",
  "auth_data_file": "/etc/vbackup/auth_data.json",
  "savedata_in_store": false,
  "reporting_file": "/etc/vbackup/reporting.json",
  "docker_images": "/etc/vbackup/images"
}
```

### Timeframes
Default file: `/etc/vbackup/timeframes.json`

| Key        | Required | Default | Description                               |
|------------|----------|---------|-------------------------------------------|
| identifier | yes      |         | The unique identifier for this timeframe. |
| interval   | yes      |         | Length of this timeframe in seconds.      |

```json
{
  "DAILY": {
    "identifier": "DAILY",
    "interval": 86400
  },
  "WEEKLY": {
    "identifier": "WEEKLY",
    "interval": 604800
  }
}
```
### Volumes
Default directory: `/etc/vbackup/volumes`. This is the configuration file for a source to back up.

| Key                | Required | Default         | Description                                                                                                                                                            |
|--------------------|----------|-----------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| name               | yes      |                 | A unique name for this configuration. Filename is recommended.                                                                                                         |
| disabled           | no       | false           | Flag to disable this configuration.                                                                                                                                    |
| source_path        | yes      |                 | Path of the directory or name of the docker volume to back up, alternatively a list of paths with name mappings.                                                       |
| source_path[].path | no       |                 | Path to a directory / name of a docker volume to back up.                                                                                                              |
| source_path[].name | no       |                 | Name for the backup up location (Used for example as a top-level directory in an archive).                                                                             |
| backup_path        | no       | $save_dir/$name | Path to store backups in. This path will be synced if both backup and sync are configured. Uses the backup directory and the name of the configuration by default.     |
| savedata_in_store  | no       | false           | Whether to store the savedata file with the backup or not. Overwrites the global flag if set.                                                                          |
| backup             | no       |                 | The backup configuration for this volume.                                                                                                                              |
| sync               | no       |                 | The sync configuration for this volume.                                                                                                                                |
| setup              | no       |                 | Options to prepare the run. Used for backup if given, else for sync, as the sync after a backup is expected to not be related to live data but already backed up data. |
| setup.containers   | no       | []              | Stop these containers before the run and restart them afterwards. Stop in the given order and start in reverse order.                                                  |
| setup.before       | no       | []              | Execute these scripts before the run. Passed to `sh -c`                                                                                                                |
| setup.after        | no       | []              | Execute these scripts after the run. Passed to `sh -c`                                                                                                                 |

```json
{
  "name": "my-important-volume",
  "disabled": false,
  "source_path": "important-volume",
  "backup_path": "/var/vbackup/my-important-volume",
  "savedata_in_store": false,
  "backup": { ... },
  "sync": { ... }
}
```

```json
{
  "source_path": [
    {
      "path": "imporant-volume",
      "name": "some-name"
    },
    {
      "path": "/path/to/a/directory",
      "name": "another-name"
    }
  ]
}
```

#### Backup
| Key                 | Required | Default | Description                                                                                                              |
|---------------------|----------|---------|--------------------------------------------------------------------------------------------------------------------------|
| disabled            | no       | false   | Flag to disable the backup configuration.                                                                                | 
| type                | yes      |         | The type of this backup configuration / which backup module to use.                                                      |
| config              | yes      |         | The module specific backup configuration.                                                                                |
| check               | no       |         | Configuration of an additional check for this backup.                                                                    |
| timeframes          | yes      |         | Timeframes in which to run this backup.                                                                                  |
| timeframes[].frame  | yes      |         | Identifier of the referenced timeframe.                                                                                  |
| timeframes[].amount | no       | 1       | The number of backups to keep for this timeframe.                                                                        |
| setup               | no       |         | Options to prepare the backup run. Overwrites the general configuration.                                                 |
| setup.containers    | no       | []      | Stop these containers before the backup and restart them afterwards. Stop in the given order and start in reverse order. |
| setup.before        | no       | []      | Execute these scripts before the backup. Passed to `sh -c`                                                               |
| setup.after         | no       | []      | Execute these scripts after the backup. Passed to `sh -c`                                                                |

```json
{
  "disabled": false,
  "type": "tar7zip",
  "config": { ... },
  "check": { ... },
  "timeframes": [
    {
      "frame": "DAILY",
      "amount": 5
    }
  ]
}
```
#### Sync
| Key              | Required | Default | Description                                                                                                            |
|------------------|----------|---------|------------------------------------------------------------------------------------------------------------------------|
| disabled         | no       | false   | Flag to disable the sync configuration.                                                                                |
| type             | yes      |         | The type of this sync configuration / which sync module to use.                                                        |
| config           | yes      |         | The module specific sync configuration.                                                                                |
| check            | no       |         | Configuration of an additional check for this sync.                                                                    |
| controller       | no       |         | Configuration of an controller for the remote device.                                                                  |
| interval         | yes      |         | The timeframe to run this sync in.                                                                                     |
| interval.frame   | yes      |         | The identifier of the referenced timeframe.                                                                            |
| setup            | no       |         | Options to prepare the sync run. Overwrites the general configuration.                                                 |
| setup.containers | no       | []      | Stop these containers before the sync and restart them afterwards. Stop in the given order and start in reverse order. |
| setup.before     | no       | []      | Execute these scripts before the sync. Passed to `sh -c`                                                               |
| setup.after      | no       | []      | Execute these scripts after the sync. Passed to `sh -c`                                                                |

```json
{
  "disabled": false,
  "type": "rsync-ssh",
  "config": { ... },
  "check": { ... },
  "controller": { ... },
  "interval": {
    "frame": "WEEKLY"
  }
}
```
### Reporting
Default file: `/etc/vbackup/reporting.json`. Send information to additional destinations, currently only MQTT.
```json
[
  {
    "type": "mqtt",
    ...
  }
]
```
### Shared authentication
Default file: `/etc/vbackup/auth_data.json`. Define configuration objects that can be 
referenced from volume configurations, this is useful if different sources are transferred
to the same destination.
```json
{
  "some-ssh-login": {
    "hostname": "my-ssh-server.local",
    "port": 22,
    "user": "foo",
    "password": "hackme",
    "ssh_key": "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\n...\n",
    "host_key": "ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMjDsazoYSf3uTT5G8YPqGJq3Hgx/YmUdCDdemWOWg+H",
    "fingerprint": "ssh-rsa 2048 4d:fc:4e:4c:c7:b4:1f:78:f6:1f:42:7b:56:69:c1:85"
  }, 
  "another_login_mqtt": {
    "host": "mqtt-broker.local",
    "port": 1883,
    "user": "user",
    "password": "hackme",
    "qos": 2
  }
}
```

## Base conditions

## Modularity
Todo: module-data, savedata.json
### Backup
#### tar7zip
Create a compressed backup in a `.tar.7z` archive.

| Key            | Required | Default | Description                                                                                                                                                                                                                          |
|----------------|----------|---------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| encryption_key | false    |         | Encrypt archives with this key.                                                                                                                                                                                                      |
| exclude[]      | false    |         | Exclude files from the archive. Uses `tar --exclude`, e.g. `./foo` refers to a directory named `foo` in the root of the saved path. If multiple locations on the host system are mapped with docker refer to those via `./name/foo`. |

```json
{
  "type": "tar7zip",
  "encryption_key": "passw0rd",
  "exclude": [
    "./some/path/to/exclude",
    "last/part/of/path/to/file.txt"
  ]
}
```

#### borg
Create a backup in a local directory using borg. The backup repository can be initiated on first run and pruned afterwards.
The backup is written to the default or specified backup path from the main backup configuration.
If init fails for some reason remove the `init-marker` file in the module data directory, which can be found be default in
`/var/vbackup/.module-data/$name/backup`.
If you want to reset the repository entirely remove the module data directory and the backup path, which usually is
`/var/vbackup/$name`.

| Key                | Required | Default | Description                                                                                                                                                                                 |
|--------------------|----------|---------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| encryption_key     | no       |         | Use this passphrase for encryption.                                                                                                                                                         |
| authentication_key | no       |         | Use this passphrase for authentication. Not used if encryption_key is set.                                                                                                                  |
| blake2             | no       | false   | Use BLAKE2b for hashing / encryption instead of SHA256. See borg documentation for details. Speed depends on the processor.                                                                 |
| quota              | no       |         | Set the storage quota when initializing the borg repo.                                                                                                                                      |
| no_init            | no       | false   | Do not initialize the borg repo. Make sure the repo is initialized and the metadata is present.                                                                                             |
| append_only        | no       | false   | Initialize repo in append-only mode.                                                                                                                                                        |
| exclude            | no       | []      | Exclude these patterns from the backup. See the borg documentation for details.                                                                                                             | 
| additional_options | no       | []      | Pass additional options to the borg create command.                                                                                                                                         | 
| disable_prune      | no       | false   | Do not prune the repo after creating a backup.                                                                                                                                              |
| prefix             | no       |         | Specify a prefix for the backup, appended after `vbackup_`. Note that the prefix will also be used for pruning, meaning that in case the prefix is changed, not all backups will be pruned. |
| relocate_ok        | no       | false   | Allow the repository location to change. If false, the backup fails on a changed destination. It can be set once to move a repo.                                                            | 
| keep               | yes      |         | Configure prune behaviour. At least one of these options is required.                                                                                                                       |
| keep.within        | no       |         | Keep all archives within this time interval. E.g. 10d for ten days.                                                                                                                         |
| keep.secondly      | no       |         | Number of secondly backups to keep.                                                                                                                                                         |
| keep.minutely      | no       |         | Number of minutely backups to keep.                                                                                                                                                         |
| keep.hourly        | no       |         | Number of hourly backups to keep.                                                                                                                                                           |
| keep.daily         | no       |         | Number of daily backups to keep.                                                                                                                                                            |
| keep.weekly        | no       |         | Number of weekly backups to keep.                                                                                                                                                           |
| keep.monthly       | no       |         | Number of monthly backups to keep.                                                                                                                                                          |
| keep.yearly        | no       |         | Number of yearly backups to keep.                                                                                                                                                           |

```json
{
  "type": "borg",
  "encryption_key": "hackme",
  "quota": "2G",
  "keep": {
    "daily": 7,
    "weekly": 4,
    "monthly": 3
  }
}
```

### Synchronization
#### rsync-ssh
Send the backup to a remote destination using rsync over ssh.

| Key                | Required | Default      | Description                                                                                                                                                                                                  |
|--------------------|----------|--------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| compress           | no       | false        | Compress the files before transmitting.                                                                                                                                                                      |
| path_prefix        | no       |              | Prefix for the remote path. Treated as a path relative to the home directory unless there is a '/' as the first character.                                                                                   |
| dirname            | yes      |              | Directory to sync to on the server. Should be only the name of the directory, not the path.                                                                                                                  |
| detect_renamed     | no       | false        | Enable the rsync detect-renamed patch. Only works if the patch is installed on client and server. If running with docker a patched version is used automatically.                                            |
| detect_renamed_lax | no       | false        | Enable the rsync detect-renamed-lax patch. Same notes as for detect_renamed.                                                                                                                                 |
| detect_moved       | no       | false        | Enable the rsync detect-moved patch. Same notes as for detect_renamed.                                                                                                                                       |
| chmod_perms        | no       | D0775,F0664  | File and directory modes to apply to written files and directories, according to the '--chmod' option of rsync.                                                                                              |
| local_chmod        | no       | $chmod_perms | Overwrite value for 'chmod_perms' when syncing to the local filesystem.                                                                                                                                      |
| remote_chmod       | no       | $chmod_perms | Overwrite value for 'chmod_perms' when syncing to the remote filesystem.                                                                                                                                     |
| local_chown        | no       |              | Owner and group for files and directories copied to the local filesystem, according to the '--chown' option of rsync. It is recommended to use the UID/GID when using docker mode, as names are not present. |
| filter             | no       |              | Set a list of filter rules according to rsync 'FILTER RULES'. Paths anchored at the root need to be prefixed with the dirname and a leading '/'. Same for in-/exclude.                                       |
| include            | no       |              | Include only the list of specified files according to rsync 'INCLUDE/EXCLUDE PATTERN RULES'. As a side-effect does not copy empty directories. Uses filter rules internally.                                 |
| exclude            | no       |              | Exclude the list of specified files according to rsync 'INCLUDE/EXCLUDE PATTERN RULES'. Uses filter rules internally.                                                                                        |
| local_rsync        | no       | rsync        | Path to the local rsync executable. When using docker:  A different image is built for detect-renamed(-lax)/detect-moved where '/usr/bin/rsync' is standard rsync and 'rsync' is the patched version.        |
| remote_rsync       | no       |              | Path to the remote rsync executable. Default is set by rsync.                                                                                                                                                |
| additional_args    | no       |              | Additional arguments for rsync.                                                                                                                                                                              |
| host_reference     | depends  |              | Reference to ssh server information in the shared authentication store.                                                                                                                                      |
| host               | depends  |              | Authentication for the ssh server. Note: Either this or the `host_reference` has to be provided.                                                                                                             | 
| host.hostname      | yes      |              | Hostname of the server.                                                                                                                                                                                      |
| host.port          | no       | 22           | Port of the server.                                                                                                                                                                                          |
| host.user          | yes      |              | Username for login on the server.                                                                                                                                                                            |
| host.password      | no       |              | Password for login on the server.                                                                                                                                                                            |
| host.ssh_key       | no       |              | Unencrypted private key for login on the server. This will be preferred over the password if both are given.                                                                                                 |
| host.host_key      | yes      |              | Public key of the host for host authentication.                                                                                                                                                              |
| host.raw_host_key  | no       | false        | Use the provided host key as a raw known_hosts file entry and do not try to prepend the appropriate hostname / port.                                                                                         | 

```json
{
  "type": "rsync-ssh",
  "compress": false,
  "path_prefix": "/home/foo",
  "dirname": "my-backup-dir",
  "detect_renamed": true,
  "local_chmod": "0000,Dug+rwx,Fug+rw,o-rwx",
  "local_chown": "1000:1000",
  "local_rsync": "/path/to/executable/rsync",
  "include": [
    "/my-backup-dir/some-dir-in-sync-root/***",
    "*.txt"
  ],
  "additional_args": [
    "--omit-dir-times"
  ],
  "host": {
    "hostname": "my-ssh-server.local",
    "port": 22,
    "user": "foo",
    "password": "bar",
    "ssh_key": "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\n...\n",
    "host_key": "ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMjDsazoYSf3uTT5G8YPqGJq3Hgx/YmUdCDdemWOWg+H"
  }
}
```

#### borg (over ssh)
Create a backup using borg over ssh. The backup repository can be initiated on first run and pruned afterwards.
Requires borg (borgbackup) to be installed on the remote machine.
If init fails for some reason remove the `init-marker` file in the module data directory, which can be found be default in 
`/var/vbackup/.module-data/$name/sync`.
If you want to reset the repository entirely remove the module data directory and the synced path on the remote machine.

| Key                | Required | Default | Description                                                                                                                                                                                 |
|--------------------|----------|---------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| directory          | yes      |         | The remote directory containing the borg repository, or the directory that should be used for it.                                                                                           |
| host_reference     | depends  |         | Reference to ssh server information in the shared authentication store.                                                                                                                     |
| host               | depends  |         | Authentication for the ssh server. Note: Either this or the `host_reference` has to be provided.                                                                                            | 
| host.hostname      | yes      |         | Hostname of the server.                                                                                                                                                                     |
| host.port          | no       | 22      | Port of the server.                                                                                                                                                                         |
| host.user          | yes      |         | Username for login on the server.                                                                                                                                                           |
| host.password      | no       |         | Password for login on the server.                                                                                                                                                           |
| host.ssh_key       | no       |         | Unencrypted private key for login on the server. This will be preferred over the password if both are given.                                                                                |
| host.host_key      | yes      |         | Public key of the host for host authentication.                                                                                                                                             |
| host.raw_host_key  | no       | false   | Use the provided host key as a raw known_hosts file entry and do not try to prepend the appropriate hostname / port.                                                                        |
| encryption_key     | no       |         | Use this passphrase for encryption.                                                                                                                                                         |
| authentication_key | no       |         | Use this passphrase for authentication. Not used if encryption_key is set.                                                                                                                  |
| blake2             | no       | false   | Use BLAKE2b for hashing / encryption instead of SHA256. See borg documentation for details. Speed depends on the processor.                                                                 |
| quota              | no       |         | Set the storage quota when initializing the borg repo.                                                                                                                                      |
| no_init            | no       | false   | Do not initialize the borg repo. Make sure the repo is initialized and the metadata is present.                                                                                             |
| append_only        | no       | false   | Initialize repo in append-only mode.                                                                                                                                                        |
| exclude            | no       | []      | Exclude these patterns from the backup. See the borg documentation for details.                                                                                                             | 
| additional_options | no       | []      | Pass additional options to the borg create command.                                                                                                                                         | 
| disable_prune      | no       | false   | Do not prune the repo after creating a backup.                                                                                                                                              |
| prefix             | no       |         | Specify a prefix for the backup, appended after `vbackup_`. Note that the prefix will also be used for pruning, meaning that in case the prefix is changed, not all backups will be pruned. |
| relocate_ok        | no       | false   | Allow the repository location to change. If false, the sync fails on a changed destination. It can be set once to move a repo.                                                              | 
| keep               | yes      |         | Configure prune behaviour. At least one of these options is required.                                                                                                                       |
| keep.within        | no       |         | Keep all archives within this time interval. E.g. 10d for ten days.                                                                                                                         |
| keep.secondly      | no       |         | Number of secondly backups to keep.                                                                                                                                                         |
| keep.minutely      | no       |         | Number of minutely backups to keep.                                                                                                                                                         |
| keep.hourly        | no       |         | Number of hourly backups to keep.                                                                                                                                                           |
| keep.daily         | no       |         | Number of daily backups to keep.                                                                                                                                                            |
| keep.weekly        | no       |         | Number of weekly backups to keep.                                                                                                                                                           |
| keep.monthly       | no       |         | Number of monthly backups to keep.                                                                                                                                                          |
| keep.yearly        | no       |         | Number of yearly backups to keep.                                                                                                                                                           |

```json
{
  "type": "borg",
  "encryption_key": "hackme",
  "remote_directory": "~/path/on/remote/server",
  "host": {
    "hostname": "my-ssh-server.local",
    "user": "foo",
    "password": "bar",
    "ssh_key": "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\n...\n",
    "host_key": "ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMjDsazoYSf3uTT5G8YPqGJq3Hgx/YmUdCDdemWOWg+H"
  },
  "prefix": "borg_backup",
  "quota": "2G",
  "keep": {
    "daily": 7,
    "weekly": 4,
    "monthly": 3
  }
}
```

#### duplicati (over sftp)
Send a backup to a remote destination using [duplicati](https://www.duplicati.com/) over sftp.
Only really makes sense without creating a local backup before.

| Key                  | Required | Default            | Description                                                                                                                                                                  |
|----------------------|----------|--------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| directory_prefix     | no       |                    | Prefix for the remote path. Treated as a path relative to the home directory unless there is a '/' as the first character.                                                   |
| directory            | yes      |                    | Directory to sync to on the server. Should be only the name of the directory, not the path.                                                                                  |
| keep_versions        | no       | 1                  | Number of versions of a file to keep. Note: Only if `smart_retention=false`.                                                                                                 |
| smart_retention      | no       | false              | Switch between smart retention policy or simple versioning.                                                                                                                  |
| retention_policy     | no       | 1W:1D,4W:1W,12M:1M | Retention policy to use. Note: Only if `smart_retention=true`. [More here.](https://duplicati.readthedocs.io/en/latest/06-advanced-options/#retention-policy)                |
| block_size           | no       | 100kb              | Size of blocks files are fragmented into. [More here.](https://duplicati.readthedocs.io/en/latest/06-advanced-options/#blocksize)                                            |
| file_size            | no       | 50mb               | Size of dblock files on the server. [More here.](https://duplicati.readthedocs.io/en/latest/06-advanced-options/#dblock-size)                                                |
| encryption_key       | no       |                    | Key to use for encrypting the backup.                                                                                                                                        |
| auth_reference       | depends  |                    | Reference to authentication information in the shared authentication store.                                                                                                  |
| auth                 | depends  |                    | Authentication for the MQTT broker. Note: Either this or the `auth_reference` has to be provided.                                                                            | 
| auth.hostname        | yes      |                    | Hostname of the server.                                                                                                                                                      |
| auth.port            | no       | 22                 | Port of the server.                                                                                                                                                          |
| auth.user            | yes      |                    | Username for login on the server.                                                                                                                                            |
| auth.password        | no       |                    | Password for login on the server.                                                                                                                                            |
| auth.ssh_key         | no       |                    | Unencrypted RSA private key for login on the server. This will be preferred over the password if both are given. Duplicati does not yet support newer SSH keys like ED25519. |
| auth.fingerprint_rsa | yes      |                    | RSA fingerprint of the server for server authentication.                                                                                                                     |

Read more about choosing file sizes (block size and file_size) [in this github issue](https://github.com/duplicati/duplicati/issues/2466)
and [this guideline by the developers](https://www.duplicati.com/articles/Choosing-Sizes/).

```json
{
  "type": "duplicati",
  "encryption_key": "supersecure",
  "directory_prefix": "/home/foo",
  "directory": "some-directory",
  "auth": {
    "hostname": "my-ssh-server.local",
    "port": "22",
    "user": "foo",
    "password": "bar",
    "ssh_key": "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\n...\n",
    "fingerprint": "ssh-rsa 2048 4d:fc:4e:4c:c7:b4:1f:78:f6:1f:42:7b:56:69:c1:85"
  },
  "smart_retention": true,
  "retention_policy": "1W:1D,4W:1W,12M:1M",
  "block_size": "100kb",
  "file_size": "50mb"
}
```

#### ssh-gpg
Tunnel files from a local directory through SSH, encrypting them with GPG in the process.
This is intended for transmitting files created by some external program, such as proxmox backups,
that are not encrypted. The local files remain unencrypted and managed by their source, while the remote backup
is encrypted and only updated when a new file is detected or an old file has been removed (only filename is checked).
This module needs to be used with care:
 - Subdirectories are not handled. Any subdirectories in the local backup directory cause this sync to fail.
 - Directory mapping is not supported. Mapped volumes would be included as directories. Only use a single local path.
 - Files with another file extension (not .gpg) in the remote directory are ignored and not removed.
 - Files in the remote directory are retrieved using `ls` over SSH, so access needs to work.
 - Access permissions (file mode) is only set on transferred files, not parent directories that may be created in the process.

| Key               | Required | Default | Description                                                                                                                            |
|-------------------|----------|---------|----------------------------------------------------------------------------------------------------------------------------------------|
| encryption_key    | yes      |         | The passphrase for gpg to encrypt backed up files.                                                                                     |
| remote_path       | yes      |         | The remote directory to save backups in.                                                                                               |
| remote_chmod      | no       |         | The file mode to set on remote files. Anything that `chmod` accepts works.                                                             |
| local_chmod       | no       |         | The file mode to set on local files. Only used for the currently unavailable `restore` operation. Anything that `chmod` accepts works. |
| host_reference    | depends  |         | Reference to ssh server information in the shared authentication store.                                                                |
| host              | depends  |         | Authentication for the ssh server. Note: Either this or the `host_reference` has to be provided.                                       | 
| host.hostname     | yes      |         | Hostname of the server.                                                                                                                |
| host.port         | no       | 22      | Port of the server.                                                                                                                    |
| host.user         | yes      |         | Username for login on the server.                                                                                                      |
| host.password     | no       |         | Password for login on the server.                                                                                                      |
| host.ssh_key      | no       |         | Unencrypted private key for login on the server. This will be preferred over the password if both are given.                           |
| host.host_key     | yes      |         | Public key of the host for host authentication.                                                                                        |
| host.raw_host_key | no       | false   | Use the provided host key as a raw known_hosts file entry and do not try to prepend the appropriate hostname / port.                   |

```json
{
  "type": "ssh-gpg",
  "encryption_key": "test12345",
  "remote_path": "/data/ssh-gpg",
  "remote_chmod": "0640",
  "host": {
    "hostname": "my-ssh-server.local",
    "port": 2222,
    "user": "foo",
    "password": "bar",
    "ssh_key": "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\n...\n",
    "host_key": "[my-ssh-server.local]:2222 ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMjDsazoYSf3uTT5G8YPqGJq3Hgx/YmUdCDdemWOWg+H"
  }
}
```

### Conditions
Additional conditions to check before creating a backup or running a sync.
Those checks are always applied additionally to timeframes.

#### file-age
Check if the newest file in the source directory has been created / edited after the last run.
Can prevent backup and sync runs on unchanged data.

| Key       | Required | Default | Description                                                                                                                                                                                                                                                                |
|-----------|----------|---------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| exclude[] | false    |         | Exclude files from the check. Uses `grep -F` to mimic basic functionality of `tar --exclude` to be somewhat compatible with `tar7zip` backup. This means input is not interpreted as a regular expression and only as a string. `./` matches the root of the checked path. |  

```json
{
  "type": "file-age",
  "exclude": [
    "./some/path/to/exclude",
    "last/part/of/path/to/file.txt"
  ]
}
```

#### usetime

Used for creating backups of game servers like minecraft and factorio.
Checks the time players have spent on the server by reading the usetime from a file 
updated by an external program, like a server plugin.

| Key              | Required | Default | Description                                                                                                                       |
|------------------|----------|---------|-----------------------------------------------------------------------------------------------------------------------------------|
| json             | no       | true    | Use json format. Json should contain a property 'usetime'. Alternatively the file is expected to contain lines like 'usetime=10'. |
| file             | yes      |         | The path to the file containing the backup information (usetime).                                                                 |
| targeted_usetime | yes      |         | The time in seconds the server has to be in use before a backup is run.                                                           |  

```json
{
  "type": "usetime",
  "backup_info": "/var/servermanager/usetime.json",
  "targeted_usetime": 3600
}
```

### Controller
Controller for starting remote devices for syncs and stopping them afterwards.

#### mqtt
Use the [MQTT device controller](https://github.com/lunarys/mqtt-device-controller) to start and stop devices for the sync.
This controller can be bundled for different configurations: 
If the configuration matches for some configurations, they are executed right after another, so that the controller only sends the start command once to the server.

| Key            | Required | Default                      | Description                                                                                       |
|----------------|----------|------------------------------|---------------------------------------------------------------------------------------------------|
| start          | no       | true                         | Wether to start the remote device or not.                                                         |
| device         | yes      |                              | Name of the remote device. (Used in default topics)                                               |
| topic_sub      | no       | device/%d/controller/to/%u   | Topic to receive messages from the controller on.                                                 |
| topic_pub      | no       | device/%d/controller/from/%u | Topic to send messages to the controller on.                                                      |
| auth_reference | depends  |                              | Reference to authentication information in the shared authentication store.                       |
| auth           | depends  |                              | Authentication for the MQTT broker. Note: Either this or the `auth_reference` has to be provided. |
| auth.host      | yes      |                              | Hostname of the MQTT broker.                                                                      |
| auth.port      | no       | 1883                         | Port of the MQTT broker.                                                                          |
| auth.user      | yes      |                              | Username for the MQTT broker.                                                                     |
| auth.password  | no       |                              | Password for the MQTT broker.                                                                     |
| auth.qos       | no       | 1                            | Quality of Service for MQTT messages.                                                             | 

```json
{
  "type": "mqtt",
  "start": true,
  "device": "sundavar",
  "topic_sub": "device/sundavar/controller/to/user",
  "topic_pub": "device/sundavar/controller/from/user",
  "auth": {
    "host": "mqtt-broker.local",
    "port": 1883,
    "user": "user",
    "password": "hackme",
    "qos": 2
  }
}
```

#### ping
Ping the specified host before attempting to sync in order to determine whether it is online.

| Key     | Required | Default | Description                                |
|---------|----------|---------|--------------------------------------------|
| address | yes      |         | The IP address or hostname to ping.        |
| timeout | no       | 10      | A timeout for the ping request in seconds. |

### Reporting
Send information about backup and sync runs to additional destinations.

#### mqtt
| Key            | Required | Default           | Description                                                                                                                                                        |
|----------------|----------|-------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| base_topic     | yes      | device/%u/vbackup | Topic that is used as a base for all messages, the specific report submodule is appended. Note: %u = MQTT user (assuming the device logs in as a designated user). |
| auth_reference | depends  |                   | Reference to authentication information in the shared authentication store.                                                                                        |
| auth           | depends  |                   | Authentication for the MQTT broker. Note: Either this or the `auth_reference` has to be provided.                                                                  |
| auth.host      | yes      |                   | Hostname of the MQTT broker.                                                                                                                                       |
| auth.port      | no       | 1883              | Port of the MQTT broker.                                                                                                                                           |
| auth.user      | yes      |                   | Username for the MQTT broker.                                                                                                                                      |
| auth.password  | no       |                   | Password for the MQTT broker.                                                                                                                                      |
| auth.qos       | no       | 1                 | Quality of Service for MQTT messages.                                                                                                                              |

```json
{
  "type": "mqtt",
  "base_topic": "device/user/vbackup",
  "auth_reference": "",
  "auth": {
    "host": "mqtt-broker.local",
    "port": 1883,
    "user": "user",
    "password": "hackme",
    "qos": 2
  }
}
```

## Todo
- Implement automatic restore of backups