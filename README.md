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
| -n, --name | no | | Name of a specific configuration to run operation on |
| -c, --config | no | /etc/vbackup/config.json | Specify the base configuration file |
| --dry-run | yes | false | Do not actually perform any permanent changes |
| -v, --verbose | yes | false | Enable verbose logging (Trace) |
| -d, --debug | yes | false | Enable debug logging (Debug) |
| -q, --quiet | yes | false | Disable info logging (Warn) |
| -f, --force | yes | false | Disregard all constraints, forcing the run |
| -b, --bare, --no-docker | yes | false | Do not use docker, warning: Can't backup docker volumes then and might affect resulting backup |    

## Configuration
### Base configuration
### Timeframes
### Volumes
### Reporting
### Shared authentication

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
    "port": "22",
    "user": "foo",
    "password": "bar",
    "ssh_key": "TODO",
    "host_key": "TODO"
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
| retention_policy | no | 1W:1D,4W:1W,12M:1M | Retention policy to use. Note: Only if `smart_retention=true`.
| block_size | no | 100kb | Size of blocks files are fragmented into. [More here.](https://duplicati.readthedocs.io/en/latest/06-advanced-options/#blocksize)
| file_size | no | 50mb | Size of dblock files on the server. [More here.](https://duplicati.readthedocs.io/en/latest/06-advanced-options/#dblock-size) 
| encryption_key | no | | Key to use for encrypting the backup. |
| auth_reference | depends | | Reference to authentication information in the shared authentication store. |
| auth | depends | | Authentication for the MQTT broker. Note: Either this or the `auth_reference` has to be provided. | 
| auth.hostname | yes | | Hostname of the server.
| auth.port | yes | | Port of the server. |
| auth.user | yes | | Username for login on the server. |
| auth.password | no | | Password for login on the server. |
| auth.ssh_key | no | | Unencrypted private key for login on the server. This will be preferred over the password if both are given. |  
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
    "ssh_key": "TODO",
    "fingerprint_rsa": "TODO"
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
| auth_reference | depends | | Reference to authentication information in the shared authentication store. |
| topic_sub | no | device/%d/controller/to/%u | Topic to receive messages from the controller on. |
| topic_pub | no | device/%d/controller/from/%u | Topic to send messages to the controller on. |
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
  "auth_reference": "",
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
    "password": "hackme,"
    "qos": 2
  }
}
```

## Todo
- Implement automatic restore of backups