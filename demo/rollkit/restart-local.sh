DA_BLOCK_HEIGHT=1
NAMESPACE=01020304050607080910111213141516
AUTH_TOKEN=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJBbGxvdyI6WyJwdWJsaWMiLCJyZWFkIiwid3JpdGUiLCJhZG1pbiJdfQ.mj7taF7Z9ZcTN2hhC1-cLf5SmqQd-ZA4YVZymd3-Ato
gmd start --rollkit.aggregator true --rollkit.da_layer sugondat --rollkit.da_config='{"base_url":"http://localhost:10995","namespace":"01020304050607080910111213141516"}' --rollkit.namespace_id $NAMESPACE --rollkit.da_start_height $DA_BLOCK_HEIGHT --rpc.laddr tcp://127.0.0.1:36657 --p2p.laddr "0.0.0.0:36656"
