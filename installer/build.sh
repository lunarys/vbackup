#!/bin/bash

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
echo "Done"