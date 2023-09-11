#!/bin/bash

BASEDIR=$(dirname $0)
if [ "$1" = "develop" ]; then
    source $BASEDIR/../../integration-tests/.contracts
    echo $SLENDER_POOL
elif [ "$1" = "futurenet" ]; then
    source $BASEDIR/../artifacts/.contracts
    echo $SLENDER_POOL
fi
source $BASEDIR/.$1.env

invoke() {
    local result=$(soroban --verbose contract invoke \
        --source $2 \
        --id $1 \
        --rpc-url "$SOROBAN_RPC_URL" \
        --network-passphrase "$PASSPHRASE" \
        -- \
        $3)
    echo $result
}

install() {
    local hash=$(soroban contract install \
        --wasm $1 \
        --source $2 \
        --rpc-url "$SOROBAN_RPC_URL" \
        --network-passphrase "$PASSPHRASE")
    echo $hash
}

POOL_HASH=$(install "../target/wasm32-unknown-unknown/release/pool.wasm" $ADMIN_SECRET)
echo "Pool wasm hash: $POOL_HASH"
echo "$POOL_HASH">$BASEDIR/../artifacts/pool.wasm.upgrades.hash

invoke $SLENDER_POOL $ADMIN_SECRET "upgrade \
    --new_wasm_hash $POOL_HASH"
