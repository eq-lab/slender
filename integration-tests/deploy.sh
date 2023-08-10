#!/bin/bash

# TODO: Fix /Artur
source .develop.env

curl -s "$FRIENDBOT_URL?addr=$ADMIN_PUBLIC" 1>/dev/null
sleep 10

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

addressFromResult() {
    IFS=',' read -ra values <<<"$(sed 's/\[\|\]//g' <<<"$1")"
    local value1="$(echo "${values[0]}" | tr -d '[:space:]' | sed 's/"//g' | sed 's/\[//g')"
    local value2="$(echo "${values[1]}" | tr -d '[:space:]')"

    echo $value1
}

TOKEN=$(deploy "../contracts/soroban_token_contract.wasm" $ADMIN_SECRET)
echo "Token contract address: $TOKEN"

DEPLOYER=$(deploy "../target/wasm32-unknown-unknown/release/deployer.wasm" $ADMIN_SECRET)
echo "Deployer contract address: $DEPLOYER"

S_TOKEN_HASH=$(install "../target/wasm32-unknown-unknown/release/s_token.wasm" $ADMIN_SECRET)
echo "SToken wasm hash: $S_TOKEN_HASH"

DEBT_TOKEN_HASH=$(install "../target/wasm32-unknown-unknown/release/s_token.wasm" $ADMIN_SECRET)
echo "DebtToken wasm hash: $DEBT_TOKEN_HASH"

POOL_HASH=$(install "../target/wasm32-unknown-unknown/release/pool.wasm" $ADMIN_SECRET)
echo "Pool wasm hash: $POOL_HASH"

PRICE_FEED=$(deploy "../target/wasm32-unknown-unknown/release/price_feed_mock.wasm" $ADMIN_SECRET)
PRICE_FEED=$(addressFromResult $PRICE_FEED)
echo "Price Feed contract address: $PRICE_FEED"

contracts=".contracts"
{
    echo "SLENDER_TOKEN=$TOKEN"
    echo "SLENDER_DEPLOYER=$DEPLOYER"
    echo "SLENDER_PRICE_FEED=$PRICE_FEED"
    echo "SLENDER_POOL_HASH=$POOL_HASH"
    echo "SLENDER_S_TOKEN_HASH=$S_TOKEN_HASH"
    echo "SLENDER_DEBT_TOKEN_HASH=$DEBT_TOKEN_HASH"
} >$contracts

echo "Contracts deployed"
