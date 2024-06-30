#!/bin/bash

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "Cargo is required but not found. Please install Rust toolchain (including cargo) to proceed."
    exit 1
fi

# Check if running as root
if [[ $EUID -eq 0 ]]; then
    echo "This script should not be run as root."
    exit 1
fi

# Determine the configuration directory
if [[ -n "$XDG_CONFIG_DIRS" ]]; then
    config_dir="${XDG_CONFIG_DIRS}/anyrun"
else
    config_dir="/etc/xdg/anyrun"
fi

if [[ -n "$XDG_DATA_HOME" ]]; then
    user_data_dir="${XDG_DATA_HOME}"
else
    user_data_dir="${HOME}/.local/share"
fi

glib_schemas_dir="${user_data_dir}/glib-2.0/schemas"

# Clone the repository
# git clone --recursive https://github.com/bzglve/anyrun.git || { echo "Failed to clone repository"; exit 1; }
# cd anyrun || { echo "Failed to change directory"; exit 1; }

# Build all packages
cargo build --release || { echo "Cargo build failed"; exit 1; }

# Install the anyrun binary
cargo install --path anyrun/ || { echo "Cargo install failed"; exit 1; }

mkdir -p $glib_schemas_dir
cp settings/1/* $glib_schemas_dir
glib-compile-schemas $glib_schemas_dir

# Create the config directory and the plugins subdirectory
sudo mkdir -p "${config_dir}/plugins" || { echo "Failed to create config directory"; exit 1; }

# Copy all of the built plugins to the correct directory
sudo cp target/release/*.so "${config_dir}/plugins" || { echo "Failed to copy plugins"; exit 1; }

# Copy the default config file
sudo cp examples/config.ron "${config_dir}/config.ron" || { echo "Failed to copy config file"; exit 1; }

echo "Setup completed successfully."

