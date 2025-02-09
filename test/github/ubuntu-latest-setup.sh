#!/bin/bash

set -euxo pipefail

sudo useradd -m testuser
echo 'testuser:testpass' | sudo chpasswd

sudo apt update
sudo apt install -y dante-server net-tools

DEFAULT_INTERFACE=$(ip route | grep default | awk '{print $5}')
sed -i "s/external: eth0/external: $DEFAULT_INTERFACE/g" ./test/dante_no_auth.conf
sed -i "s/external: eth0/external: $DEFAULT_INTERFACE/g" ./test/dante_password.conf

sudo danted -D -f ./test/github/dante_no_auth.conf
sudo danted -D -f ./test/github/dante_password.conf

sleep 10

netstat -tunlp
