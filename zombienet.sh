#!/bin/bash

# Check if the zombienet binary is available.
if ! [ -x "$(command -v zombienet)" ]; then
  echo "\
zombienet is not found in PATH. Install zombienet.

Available at https://github.com/paritytech/zombienet"
  exit 1
fi

if ! [ -x "$(command -v polkadot)" ]; then
  echo "\
'polkadot' is not found in PATH. Install polkadot.

To obtain, refer to https://github.com/paritytech/polkadot-sdk/tree/master/polkadot#polkadot"
    exit 1
fi

if ! [ -x "$(command -v sugondat-node)" ]; then
  echo "\
'sugondat-node' is not found in PATH. cd to 'sugondat/chain' and run 'cargo build --release'
and add the result into your PATH."
  exit 1
fi

zombienet spawn -p native --dir zombienet testnet.toml
