#!/bin/bash
set -e

if [[ "$1" == "--help" ]] || [[ "$1" == "-h" ]]; then
    echo "Usage: './install.sh [--update-only]'"
    echo "Use the option 'update-only' to only update the executable and leave the configuration files untouched"
    exit 0
fi

# Requires root privileges
if [[ "$UID" != 0 ]]; then
    echo "Please run this installer as root..."
    exit 1
fi

# Primitive check to determine the current directory
if [[ ! -e ./install.sh ]]; then
    echo "Please execute this script from the installer directory..."
    exit 2
fi

# Check if docker is installed
if ! which docker &> /dev/null; then
    echo "Please install docker to use this installer..."
    exit 3
fi

UPDATE_ONLY=false
if [[ "$1" == "--update" ]] || [[ "$1" == "-u" ]] || [[ "$1" == "--update-only" ]]; then
    UPDATE_ONLY=true
fi

# Check if image exists (is built)
DOCKER_FILE="cargo.Dockerfile"
DOCKER_IMAGE="my-rust-compiler"
if [[ "$(docker images -q ${DOCKER_IMAGE} 2> /dev/null)" == "" ]]; then
    echo "Building compiler docker image..."
    docker build -t ${DOCKER_IMAGE} -f ${DOCKER_FILE} .
fi

# Compile vbackup
echo "Compiling vbackup..."
docker run --rm --name="vbackup-compiler" --volume="$(pwd)/..:/project" -w /project ${DOCKER_IMAGE} cargo build --release

# Create directories
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/vbackup"
if ! ${UPDATE_ONLY}; then
    echo "Creating directories..."
    [[ ! -d ${CONFIG_DIR} ]] && mkdir ${CONFIG_DIR}
fi

# Copy required files
echo "Copying required files..."
EXECUTABLE="$INSTALL_DIR/vbackup"
cp ../target/release/vbackup ${EXECUTABLE}
if ! ${UPDATE_ONLY}; then
    mkdir "${CONFIG_DIR}/volumes"
    cp -r ../resources/images ../resources/config.json ../resources/reporting.json ../resources/auth_data.json ../resources/timeframes.json ${CONFIG_DIR}
else
    # Update timeframes config only
    cp ../resources/timeframes.json ${CONFIG_DIR}/timeframes.json
    chown root:root ${CONFIG_DIR}/timeframes.json
    chmod u+rwX,go-rwx ${CONFIG_DIR}/timeframes.json
fi

# Set permissions on files
chown root:root ${EXECUTABLE}
chmod 744 ${EXECUTABLE}
if ! ${UPDATE_ONLY}; then
    chown -R root:root ${CONFIG_DIR}
    chmod -R u+rwX,go-rwx ${CONFIG_DIR}
    chmod go+rX ${CONFIG_DIR}
fi

echo "Done!"
echo "The configuration can now be edited in '${CONFIG_DIR}'"
