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

## Running as a service
I use MQTT to trigger runs. 
For that I wrote a simple service running the executable whenever a specific message is received on specific MQTT topics. 
This ensures the synchronization on different devices is run at the same time, 
such that the backup server is not started for each sync separately and can profit from multiple syncs while it is online.

## Command line arguments
`vbackup <operation> [options]`

| Operation | Description |
|-----------|-------------|
| run       | Run backup & sync |
| backup    | Run only backup |
| sync      | Run only sync |
| list      | List all configurations |

| Option | is flag | Default value | Description |
|--------|------|:-------------:|-------------|
| -n, --name | no | | Name of a specific configuration to run operation on. |
| -c, --config | no | /etc/vbackup/config.json | Specify the base configuration file. |
| --dry-run | yes | false | Do not perform any permanent changes, instead print what would be done. |
| -v, --verbose | yes | false | Enable verbose logging (Loglevel: Trace). |
| -d, --debug | yes | false | Enable debug logging (Loglevel: Debug). |
| -q, --quiet | yes | false | Disable info logging (Loglevel: Warn). |
| -f, --force | yes | false | Disregard all constraints, forcing the run. |
| -b, --bare, --no-docker | yes | false | Do not use docker. Warning: Can't backup docker volumes (duh!) and might affect the structure of the resulting backup. Not tested thoroughly. |
| --no-reporting | yes | false | Disable reporting for this run. |    

## Configuration
### Base configuration
Default file: `/etc/vbackup/config.json`

| Key | Required | Default | Description |
|-----|----------|---------|-------------|
| config_dir | no | /etc/vbackup | Defines the base directory for all configuration files. |
| save_dir | no | /var/vbackup | Defines the base directory for all saves. |
| tmp_dir | no | /tmp/vbackup | Defines the base directory for temporary files. |
| timeframes_file | no | $base_dir/timeframes.json | Path to the file containing all timeframe definitions. |
| auth_data_file | no | $base_dir/auth_data.json | Path to the file containing all shared authentication data. |
| reporting_file | no | $base_dir/reporting.json | Path to the file containing all reporting module configurations. |
| docker_images | no | $base_dir/images | Path to the directory containing all docker files. |
| savedata_in_store | no | false | Flag for writing all savedata into the store_path of the configuration instead of the module data directory. |

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

| Key | Required | Default | Description |
|-----|----------|---------|-------------|
| identifier | yes | | The unique identifier for this timeframe. |
| interval | yes | | Length of this timeframe in seconds.

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
Default directory: `/etc/vbackup/volumes`

| Key | Required | Default | Description |
|-----|----------|---------|-------------|
| name | yes | | A unique name for this configuration. Filename is recommended. |
| disabled | no | false | Flag to disable this configuration. |
| source_path | yes | | Path of the directory or name of the docker volume to back up. |
| backup_path | no | $save_dir/$name | Path to store backups in. This path will be synced if both backup and sync are configured. Uses the backup directory and the name of the configuration by default.|
| savedata_in_store | no | false | Wether to store the savedata file with the backup or not. Overwrites the global flag if set. |
| backup | no | | The backup configuration for this volume. |
| sync | no | | The sync configuration for this volume. | 

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
#### Backup
| Key | Required | Default | Description |
|-----|----------|---------|-------------|
| disabled | no | false | Flag to disable the backup configuration. | 
| type | yes | | The type of this backup configuration / which backup module to use. |
| config | yes | | The module specific backup configuration. |
| check | no | | Configuration of an additional check for this backup. |
| timeframes | yes | | Timeframes in which to run this backup. |
| timeframes.x.frame | yes | | Identifier of the referenced timeframe. |
| timeframes.x.amount | no | 1 | The number of backups to keep for this timeframe. |

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
| Key | Required | Default | Description |
|-----|----------|---------|-------------|
| disabled | no | false | Flag to disable the sync configuration. |
| type | yes | | The type of this sync configuration / which sync module to use. |
| config | yes | | The module specific sync configuration. |
| check | no | | Configuration of an additional check for this sync. |
| controller | no | | Configuration of an controller for the remote device. |
| interval | yes | | The timeframe to run this sync in. |
| interval.frame | yes | | The identifier of the referenced timeframe. | 

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
Default file: `/etc/vbackup/reporting.json`
```json
[
  {
    "type": "mqtt",
    ...
  }
]
```
### Shared authentication
Default file: `/etc/vbackup/auth_data.json`
```json
{
  "some-ssh-login": {
    "hostname": "my-ssh-server.local",
    "port": 22,
    "user": "foo",
    "password": "hackme",
    "ssh_key": "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\n...\n",
    "host_key": "[my-ssh-server.local]:22 ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMjDsazoYSf3uTT5G8YPqGJq3Hgx/YmUdCDdemWOWg+H",
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
| Key | Required | Default | Description |
|-----|----------|---------|-------------|
| encryption_key | false | | Encrypt archives with this key. | 

```json
{
  "type": "tar7zip",
  "encryption_key": "passw0rd"
}
```

### Synchronization
#### rsync-ssh
| Key | Required | Default | Description |
|-----|----------|---------|-------------|
| compress | no | false | Compress the files before transmitting. |
| path_prefix | no | /home/%u | Prefix for the remote path. |
| dirname | yes | | Directory to sync to on the server. |
| host_reference | depends | | Reference to ssh server information in the shared authentication store. | 
| host | depends | | Authentication for the ssh server. Note: Either this or the `host_reference` has to be provided. | 
| host.hostname | yes | | Hostname of the server.
| host.port | yes | | Port of the server. |
| host.user | yes | | Username for login on the server. |
| host.password | no | | Password for login on the server. |
| host.ssh_key | no | | Unencrypted private key for login on the server. This will be preferred over the password if both are given. |
| host.host_key | yes | | Public key of the host for host authentication. |

```json
{
  "type": "rsync-ssh",
  "compress": false,
  "path_prefix": "/home/foo",
  "dirname": "my-backup-dir/sub-dir",
  "host": {
    "hostname": "my-ssh-server.local",
    "port": 22,
    "user": "foo",
    "password": "bar",
    "ssh_key": "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\n...\n",
    "host_key": "[my-ssh-server.local]:22 ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMjDsazoYSf3uTT5G8YPqGJq3Hgx/YmUdCDdemWOWg+H"
  }
}
```
#### duplicati (over sftp)
| Key | Required | Default | Description |
|-----|----------|---------|-------------|
| directory_prefix | no | |  Prefix for the remote path. |
| directory | yes | | Directory to sync to on the server. | 
| keep_versions | no | 1 | Number of versions of a file to keep. Note: Only if `smart_retention=false`.
| smart_retention | no | false | Switch between smart retention policy or simple versioning. 
| retention_policy | no | 1W:1D,4W:1W,12M:1M | Retention policy to use. Note: Only if `smart_retention=true`. [More here.](https://duplicati.readthedocs.io/en/latest/06-advanced-options/#retention-policy)
| block_size | no | 100kb | Size of blocks files are fragmented into. [More here.](https://duplicati.readthedocs.io/en/latest/06-advanced-options/#blocksize)
| file_size | no | 50mb | Size of dblock files on the server. [More here.](https://duplicati.readthedocs.io/en/latest/06-advanced-options/#dblock-size) 
| encryption_key | no | | Key to use for encrypting the backup. |
| auth_reference | depends | | Reference to authentication information in the shared authentication store. |
| auth | depends | | Authentication for the MQTT broker. Note: Either this or the `auth_reference` has to be provided. | 
| auth.hostname | yes | | Hostname of the server.
| auth.port | yes | | Port of the server. |
| auth.user | yes | | Username for login on the server. |
| auth.password | no | | Password for login on the server. |
| auth.ssh_key | no | | Unencrypted RSA private key for login on the server. This will be preferred over the password if both are given. Duplicati does not yet support newer SSH keys like ED25519.|
| auth.fingerprint_rsa | yes | | RSA fingerprint of the server for server authentication. |
```json
{
  "encryption_key": "supersecure",
  "directory_prefix": "/home/foo",
  "directory": "directory/sub-directory",
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

### Conditions
#### file-age
| Key | Required | Default | Description |
|-----|----------|---------|-------------|

```json
{
  "type": "file-age"
}
```

### Controller
#### mqtt
| Key | Required | Default | Description |
|-----|----------|---------|-------------|
| start | no | true | Wether to start the remote device or not. |
| device | yes | | Name of the remote device. (Used in default topics) |
| topic_sub | no | device/%d/controller/to/%u | Topic to receive messages from the controller on. |
| topic_pub | no | device/%d/controller/from/%u | Topic to send messages to the controller on. |
| auth_reference | depends | | Reference to authentication information in the shared authentication store. |
| auth | depends | | Authentication for the MQTT broker. Note: Either this or the `auth_reference` has to be provided. |
| auth.host | yes | | Hostname of the MQTT broker. |
| auth.port | no | 1883 | Port of the MQTT broker. |
| auth.user | yes | | Username for the MQTT broker. |
| auth.password | no | | Password for the MQTT broker. |
| auth.qos | no | 1 | Quality of Service for MQTT messages. | 

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
### Reporting
#### mqtt
| Key | Required | Default | Description |
|-----|----------|---------|-------------|
| base_topic | yes | device/%u/vbackup | Topic that is used as a base for all messages, the specific report submodule is appended. Note: %u = MQTT user (assuming the device logs in as a designated user). |
| auth_reference | depends | | Reference to authentication information in the shared authentication store. |
| auth | depends | | Authentication for the MQTT broker. Note: Either this or the `auth_reference` has to be provided. |
| auth.host | yes | | Hostname of the MQTT broker. |
| auth.port | no | 1883 | Port of the MQTT broker. |
| auth.user | yes | | Username for the MQTT broker. |
| auth.password | no | | Password for the MQTT broker. |
| auth.qos | no | 1 | Quality of Service for MQTT messages. |

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