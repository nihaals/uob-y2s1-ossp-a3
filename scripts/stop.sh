#!/usr/bin/env bash

# Removes the kernel module and the device file

set -euo pipefail

echo "Removing device file..."
sudo rm /dev/chardev || :

echo "Removing kernel module..."
sudo rmmod charDeviceDriver.ko || :
