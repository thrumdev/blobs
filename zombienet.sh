#!/bin/bash

os=$(uname -s)

if [ "$os" == "Darwin" ]; then
    ./bin/zombienet-macos spawn -p native --dir zombienet testnet.toml

elif [ "$os" == "Linux" ]; then
    # @gabriele-0201 TODO: find a better way to do this
    source export_paths.sh
    ./../zombienet-linux-x64 spawn -p native --dir zombienet testnet.toml
else
    echo "Unsupported operating system"
    exit 1
fi
