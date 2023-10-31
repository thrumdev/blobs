#!/bin/bash

SOV_CLI=/Users/pepyakin/dev/parity/sugondat-chain/sovereign-sdk/target/debug/sov-cli

$SOV_CLI \
    serialize-call \
    my_private_key.json \
    Bank \
    src/sov-cli/test_data/transfer.json \
    0

$SOV_CLI \
    make-blob src/sov-cli/test_data/transfer.dat
