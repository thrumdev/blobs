#!/bin/bash

SOV_CLI="/usr/bin/sov-cli"

# setup rpc endpoint
$SOV_CLI rpc set-url http://127.0.0.1:12345

# import keys
$SOV_CLI keys import --nickname token_deployer --path ../test-data/keys/token_deployer_private_key.json

# Create and mint a new token
$SOV_CLI transactions import from-file bank --path ../test-data/requests/create_token.json
$SOV_CLI transactions import from-file bank --path ../test-data/requests/mint.json

# submit batch with two transactions
$SOV_CLI rpc submit-batch by-nickname token_deployer

# way to let the rollup fetch from the DA the transaction and process it and then
# query the rollup node to check the correct amout of tokens were minted
sleep 30
echo "4000 tokens should be minted"
curl -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"bank_supplyOf","params":["sov1zdwj8thgev2u3yyrrlekmvtsz4av4tp3m7dm5mx5peejnesga27svq9m72"],"id":1}' http://127.0.0.1:12345
