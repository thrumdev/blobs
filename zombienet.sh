#!/bin/bash

# Check if the zombienet binary is available.
if ! [ -x "$(command -v zombienet)" ]; then
  echo "\
zombienet is not found in PATH. Install zombienet.

Available at https://github.com/paritytech/zombienet"
  exit 1
fi

zombienet spawn -p native --dir zombienet testnet.toml
