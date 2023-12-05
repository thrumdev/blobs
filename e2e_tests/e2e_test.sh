#!/bin/bash

cd "$(dirname "${BASH_SOURCE[0]}")"
PARACHAIN_ID=3338

# Expect sugondat-node to be already compiled
#./../target/release/sugondat-node export-genesis-state --parachain-id $PARACHAIN_ID > statefile
#./../target/release/sugondat-node export-genesis-wasm > wasmfile

python3 inject.py

# This could be used to kill all the running chopsick instances
# ps aux | rg np | awk 'BEGIN{FS="[   ]+"}{print $2}' | TODO | kill

# Start Kusama fork
npx @acala-network/chopsticks@latest --config=kusama_injected.yml

# To advance with block production:
# wscat -c 127.0.0.1:8000 -x '{"id": 2, "jsonrpc": "2.0", "method": "dev_newBlock", "params": [{"count": 10}] }'

# Wait for the kusama fork to start
#sleep 10
#codegen for subxt 
#subxt metadata --url ws://127.0.0.1:8000 -f bytes > kusama_metadata.scale

# Execute parachain Collators
# TODO
#./../target/release/sugondat-node \
#--alice \
#--collator \
#--force-authoring \
#--chain sugondat-kusama-staging \
#--base-path /tmp/parachain/alice \
#--port 40333 \
#--rpc-port 8844 \
#-- \
#--execution wasm \
#--chain ../polkadot/raw-local-chainspec.json \ <--????
#--port 30343 \
#--rpc-port 9977
