#!/bin/bash

# TODO: Fix /Artur
source .develop.env

# TREASURY_SECRET="${TREASURY_SECRET:-SDUQ5O67BWZUAR6GTCFWN6QM7BZBXDD5SRPMWUTIS2N37CVNKPNV3GFY}"
# TREASURY_PUBLIC="${TREASURY_PUBLIC:-GCG4IJLJBBHAAACKB245CSY6HFDQDL3OO4FCNYACQE2S7X4P36FAXT3Q}"

curl -s "$FRIENDBOT_URL?addr=$ADMIN_PUBLIC" 1>/dev/null
sleep 5

deploy() {
    local address=$(soroban contract deploy \
        --wasm $1 \
        --source $2 \
        --rpc-url "$SOROBAN_RPC_URL" \
        --network-passphrase "$PASSPHRASE")
    echo $address
}

install() {
    local hash=$(soroban contract install \
        --wasm $1 \
        --source $2 \
        --rpc-url "$SOROBAN_RPC_URL" \
        --network-passphrase "$PASSPHRASE")
    echo $hash
}

# invoke() {
#     local result=$(soroban contract invoke \
#         --id $1 \
#         --source $2 \
#         --rpc-url "$SOROBAN_RPC_URL" \
#         --network-passphrase "$PASSPHRASE" \
#         -- \
#         $3)
#     echo $result
# }

addressFromResult() {
    IFS=',' read -ra values <<<"$(sed 's/\[\|\]//g' <<<"$1")"
    local value1="$(echo "${values[0]}" | tr -d '[:space:]' | sed 's/"//g' | sed 's/\[//g')"
    local value2="$(echo "${values[1]}" | tr -d '[:space:]')"

    echo $value1
}

TOKEN=$(deploy "../contracts/soroban_token_contract.wasm" $ADMIN_SECRET)
echo "Token contract address: $TOKEN"
# invoke $TOKEN $TOKEN_SECRET "initialize \
#     --admin $TOKEN_PUBLIC \
#     --decimal 9 \
#     --name $(echo -n "token" | xxd -p) \
#     --symbol $(echo -n "TKN" | xxd -p)"

# DEBT_TOKEN=$(deploy "../contracts/soroban_token_contract.wasm" $TOKEN_SECRET)
# echo "Token contract address (debt token): $DEBT_TOKEN"
# invoke $DEBT_TOKEN $TOKEN_SECRET "initialize \
#     --admin $TOKEN_PUBLIC \
#     --decimal 9 \
#     --name $(echo -n "debt-token" | xxd -p) \
#     --symbol $(echo -n "DTKN" | xxd -p)"

DEPLOYER=$(deploy "../target/wasm32-unknown-unknown/release/deployer.wasm" $ADMIN_SECRET)
echo "Deployer contract address: $DEPLOYER"

S_TOKEN_HASH=$(install "../target/wasm32-unknown-unknown/release/s_token.wasm" $ADMIN_SECRET)
echo "SToken wasm hash: $S_TOKEN_HASH"

DEBT_TOKEN_HASH=$(install "../target/wasm32-unknown-unknown/release/s_token.wasm" $ADMIN_SECRET)
echo "DebtToken wasm hash: $DEBT_TOKEN_HASH"

POOL_HASH=$(install "../target/wasm32-unknown-unknown/release/pool.wasm" $ADMIN_SECRET)
echo "Pool wasm hash: $POOL_HASH"

# deployPoolResult=$(invoke $deployer $POOL_SECRET "deploy_pool \
#     --salt 0000000000000000000000000000000000000000000000000000000000000000 \
#     --wasm_hash $poolHash \
#     --admin $POOL_PUBLIC")
# POOL=$(addressFromResult $deployPoolResult)
# echo "Pool contract address: $POOL"

# deployStokenResult=$(invoke $deployer $TOKEN_SECRET "deploy_s_token \
#     --salt 0000000000000000000000000000000000000000000000000000000000000001 \
#     --wasm_hash $stokenHash \
#     --decimal 9 \
#     --name $(echo -n "stoken" | xxd -p) \
#     --symbol $(echo -n "STKN" | xxd -p) \
#     --pool $POOL \
#     --treasury $TREASURY_PUBLIC \
#     --underlying_asset $TOKEN")
# STOKEN=$(addressFromResult $deployStokenResult)
# echo "Stoken contract address: $STOKEN"

PRICE_FEED=$(deploy "../target/wasm32-unknown-unknown/release/price_feed_mock.wasm" $ADMIN_SECRET)
PRICE_FEED=$(addressFromResult $PRICE_FEED)
echo "Price Feed contract address: $PRICE_FEED"
# echo "Price feed contract address: $PRICE_FEED"

contracts=".contracts"
{
    echo "TOKEN=$TOKEN"
    echo "DEPLOYER=$DEPLOYER"
    echo "PRICE_FEED=$PRICE_FEED"
    echo "POOL_HASH=$POOL_HASH"
    echo "S_TOKEN_HASH=$S_TOKEN_HASH"
    echo "DEBT_TOKEN_HASH=$DEBT_TOKEN_HASH"
} >$contracts

echo "Contracts deployed"
