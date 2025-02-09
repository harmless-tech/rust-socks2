#!/bin/bash

set -euxo pipefail

eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv)"
brew install dante

DEFAULT_INTERFACE=$(route get default | grep interface | awk '{print $2}')
sed -i '' "s/external: eth0/external: $DEFAULT_INTERFACE/g" ./test/dante_no_auth.conf
sed -i '' "s/external: eth0/external: $DEFAULT_INTERFACE/g" ./test/dante_password.conf

sudo sockd -D -f ./test/github/dante_no_auth.conf
sudo sockd -D -f ./test/github/dante_password.conf

sleep 10

lsof -Pn -i
