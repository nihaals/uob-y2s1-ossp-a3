#!/usr/bin/env bash

# - Cleans up the module and device file
# - Compiles the kernel module
# - Loads the kernel module
# - Creates the device file

set -eu

./scripts/stop.sh

echo "Compiling the module..."
make clean
make

echo "Loading the module..."
sudo insmod charDeviceDriver.ko

echo "Creating the device file..."
# Sleep just in case
MAJOR_NUMBER=$(dmesg | tac | grep -Pom 1 'mknod \/dev\/chardev c \K\d+')
echo "Major number: \`$MAJOR_NUMBER\`"
sudo mknod /dev/chardev c "$MAJOR_NUMBER" 0
echo "Device file created: \`/dev/chardev\`"
sudo chown "$USER" /dev/chardev
echo "Device file ownership changed to \`$USER\`"
