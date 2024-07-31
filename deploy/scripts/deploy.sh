#!/bin/bash

BASEDIR=$(dirname $0)
BUILD=$BASEDIR/../../target/wasm32-unknown-unknown/release

source $BASEDIR/.$1.env

deploy() {
    local address
    address=$(stellar contract deploy \
        --wasm $1 \
        --source $2 \
        --rpc-url "$SOROBAN_RPC_URL" \
        --network-passphrase "$PASSPHRASE")
    echo $address
}

install() {
    local hash
    hash=$(stellar contract install \
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

if [ $1 == "mainnet" ]
then
    ARTIFACTS=$BASEDIR/../artifacts/mainnet
    
    cp $BUILD/*.wasm $ARTIFACTS

    echo "WASM files have been copied"

    find $ARTIFACTS -name \*.wasm -exec stellar contract optimize --wasm {} --wasm-out {} \; 1>/dev/null

    echo "WASM files has been optimized"

    TOKEN_XLM="CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA"
    echo "  XLM contract address: $TOKEN_XLM"

    TOKEN_XRP="CAAV3AE3VKD2P4TY7LWTQMMJHIJ4WOCZ5ANCIJPC3NRSERKVXNHBU2W7"
    echo "  XRP contract address: $TOKEN_XRP"

    TOKEN_USDC="CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75"
    echo "  USDC contract address: $TOKEN_USDC"

    DEPLOYER=$(deploy "$ARTIFACTS/deployer.wasm" $ADMIN_SECRET)
    echo "  Deployer contract address: $DEPLOYER"

    S_TOKEN_HASH=$(install "$ARTIFACTS/s_token.wasm" $ADMIN_SECRET)
    echo "  SToken wasm hash: $S_TOKEN_HASH"

    DEBT_TOKEN_HASH=$(install "$ARTIFACTS/debt_token.wasm" $ADMIN_SECRET)
    echo "  DebtToken wasm hash: $DEBT_TOKEN_HASH"

    PRICE_FEED="CALI2BYU2JE6WVRUFYTS6MSBNEHGJ35P4AVCZYF3B6QOE3QKOB2PLE6M"
    echo "  Price Feed contract address: $PRICE_FEED"

    POOL_HASH=$(install "$ARTIFACTS/pool.wasm" $ADMIN_SECRET)
    echo "  Pool wasm hash: $POOL_HASH"
else
    MOCKS=$BASEDIR/../../mocks
    ARTIFACTS=$BASEDIR/../artifacts/testnet

    cp $BUILD/*.wasm $ARTIFACTS
    cp $MOCKS/soroban_token_contract.wasm $ARTIFACTS/token.wasm

    echo "WASM files have been copied"

    find $ARTIFACTS -name \*.wasm -exec stellar contract optimize --wasm {} --wasm-out {} \; 1>/dev/null

    echo "WASM files has been optimized"

    curl -s "$FRIENDBOT_URL?addr=$ADMIN_PUBLIC" 1>/dev/null
    sleep 10

    echo "Admin's account has been funded"

    TOKEN_XLM=$(deploy "$ARTIFACTS/token.wasm" $ADMIN_SECRET)
    # TOKEN_XLM="CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"
    echo "  XLM contract address: $TOKEN_XLM"

    TOKEN_XRP=$(deploy "$ARTIFACTS/token.wasm" $ADMIN_SECRET)
    echo "  XRP contract address: $TOKEN_XRP"

    TOKEN_USDC=$(deploy "$ARTIFACTS/token.wasm" $ADMIN_SECRET)
    echo "  USDC contract address: $TOKEN_USDC"

    TOKEN_RWA=$(deploy "$ARTIFACTS/token.wasm" $ADMIN_SECRET)
    echo "  RWA contract address: $TOKEN_RWA"

    DEPLOYER=$(deploy "$ARTIFACTS/deployer.wasm" $ADMIN_SECRET)
    echo "  Deployer contract address: $DEPLOYER"

    S_TOKEN_HASH=$(install "$ARTIFACTS/s_token.wasm" $ADMIN_SECRET)
    echo "  SToken wasm hash: $S_TOKEN_HASH"

    DEBT_TOKEN_HASH=$(install "$ARTIFACTS/debt_token.wasm" $ADMIN_SECRET)
    echo "  DebtToken wasm hash: $DEBT_TOKEN_HASH"

    POOL_HASH=$(install "$ARTIFACTS/pool.wasm" $ADMIN_SECRET)
    echo "  Pool wasm hash: $POOL_HASH"

    PRICE_FEED=$(deploy "$ARTIFACTS/price_feed_mock.wasm" $ADMIN_SECRET)
    PRICE_FEED=$(addressFromResult $PRICE_FEED)
    echo "  Price Feed contract address: $PRICE_FEED"
fi

contracts="$ARTIFACTS/.contracts"
{
    echo "SLENDER_TOKEN_XLM=$TOKEN_XLM"
    echo "SLENDER_TOKEN_XRP=$TOKEN_XRP"
    echo "SLENDER_TOKEN_USDC=$TOKEN_USDC"
    echo "SLENDER_TOKEN_RWA=$TOKEN_RWA"
    echo "SLENDER_S_TOKEN_HASH=$S_TOKEN_HASH"
    echo "SLENDER_DEBT_TOKEN_HASH=$DEBT_TOKEN_HASH"
    echo "SLENDER_DEPLOYER=$DEPLOYER"
    echo "SLENDER_PRICE_FEED=$PRICE_FEED"
    echo "SLENDER_POOL_HASH=$POOL_HASH"
} >$contracts

echo "Contracts have been deployed"
