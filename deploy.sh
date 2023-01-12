RES=$(osmosisd tx wasm store artifacts/fee_abstraction.wasm --keyring-backend=test --home=$HOME/.osmosisd/validator1 --from validator1 --chain-id testing --gas 10000000 --fees 25000stake)
INIT='{"packet_lifetime":100}'
osmosisd tx wasm instantiate 4 "$INIT" --keyring-backend=test --home=$HOME/.osmosisd/validator1 --from validator1 --chain-id testing --label "test" --no-admin

CONTRACT=$(osmosisd query wasm list-contract-by-code 4 --output json | jq -r '.contracts[-1]')

query_params='{"query_stargate_twap":{"pool_id":1,"token_in_denom":"uosmo","token_out_denom":"uatom","with_swap_fee":false}}'
osmosisd query wasm contract-state smart $CONTRACT "$query_params"
query_ibc='{"query_ibc_data":{}}'