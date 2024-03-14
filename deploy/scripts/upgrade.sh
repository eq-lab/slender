#!/bin/bash

BASEDIR=$(dirname $0)
source $BASEDIR/../artifacts/.$1.contracts
echo $SLENDER_POOL
source $BASEDIR/.$1.env

invoke() {
    local result=$(soroban --verbose contract invoke \
        --id $1 \
        --source $2 \
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

POOL_HASH=$(install "/Users/maks/Work/slender/target/wasm32-unknown-unknown/release/pool.optimized.wasm" $ADMIN_SECRET)
echo "Pool wasm hash: $POOL_HASH"
echo "$POOL_HASH" >$BASEDIR/../artifacts/pool.wasm.upgrades.hash

invoke $SLENDER_POOL $ADMIN_SECRET "upgrade \
    --new_wasm_hash $POOL_HASH"
