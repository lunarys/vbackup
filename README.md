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
### Synchronization
### Conditions
### Controller
### Reporting

## Todo
- Implement automatic restore of backups