#!/bin/bash

BASEDIR=$(dirname $0)
source $BASEDIR/.$1.env

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

TOKEN_XLM=$(deploy "$BASEDIR/../../mocks/soroban_token_contract.wasm" $ADMIN_SECRET)
echo "XLM contract address: $TOKEN_XLM"

TOKEN_XRP=$(deploy "$BASEDIR/../../mocks/soroban_token_contract.wasm" $ADMIN_SECRET)
echo "XRP contract address: $TOKEN_XRP"

TOKEN_USDC=$(deploy "$BASEDIR/../../mocks/soroban_token_contract.wasm" $ADMIN_SECRET)
echo "USDC contract address: $TOKEN_USDC"

DEPLOYER=$(deploy "$BASEDIR/../../target/wasm32-unknown-unknown/release/deployer.wasm" $ADMIN_SECRET)
echo "Deployer contract address: $DEPLOYER"

S_TOKEN_HASH=$(install "$BASEDIR/../../target/wasm32-unknown-unknown/release/s_token.wasm" $ADMIN_SECRET)
echo "SToken wasm hash: $S_TOKEN_HASH"

DEBT_TOKEN_HASH=$(install "$BASEDIR/../../target/wasm32-unknown-unknown/release/debt_token.wasm" $ADMIN_SECRET)
echo "DebtToken wasm hash: $DEBT_TOKEN_HASH"

POOL_HASH=$(install "$BASEDIR/../../target/wasm32-unknown-unknown/release/pool.wasm" $ADMIN_SECRET)
echo "Pool wasm hash: $POOL_HASH"

PRICE_FEED=$(deploy "$BASEDIR/../../target/wasm32-unknown-unknown/release/price_feed_mock.wasm" $ADMIN_SECRET)
PRICE_FEED=$(addressFromResult $PRICE_FEED)
echo "Price Feed contract address: $PRICE_FEED"

contracts="$BASEDIR/../artifacts/.contracts"
{
    echo "SLENDER_TOKEN_XLM=$TOKEN_XLM"
    echo "SLENDER_TOKEN_XRP=$TOKEN_XRP"
    echo "SLENDER_TOKEN_USDC=$TOKEN_USDC"
    echo "SLENDER_S_TOKEN_HASH=$S_TOKEN_HASH"
    echo "SLENDER_DEBT_TOKEN_HASH=$DEBT_TOKEN_HASH"
    echo "SLENDER_DEPLOYER=$DEPLOYER"
    echo "SLENDER_PRICE_FEED=$PRICE_FEED"
    echo "SLENDER_POOL_HASH=$POOL_HASH"
} >$contracts

echo "Contracts have been deployed"

cp $BASEDIR/../../target/wasm32-unknown-unknown/release/*.wasm $BASEDIR/../artifacts
cp $BASEDIR/../../mocks/soroban_token_contract.wasm $BASEDIR/../artifacts/token.wasm

echo "WASM files have been copied"

soroban contract bindings typescript \
  --wasm $BASEDIR/../artifacts/debt_token.wasm \
  --output-dir $BASEDIR/../artifacts/debttoken \
  --contract-name @bindings/debttoken \
  --contract-id "" &>/dev/null

soroban contract bindings typescript \
  --wasm $BASEDIR/../artifacts/s_token.wasm \
  --output-dir $BASEDIR/../artifacts/stoken \
  --contract-name @bindings/stoken \
  --contract-id "" &>/dev/null

soroban contract bindings typescript \
  --wasm $BASEDIR/../artifacts/pool.wasm \
  --output-dir $BASEDIR/../artifacts/pool \
  --contract-name @bindings/pool \
  --contract-id "" &>/dev/null

soroban contract bindings typescript \
  --wasm $BASEDIR/../artifacts/token.wasm \
  --output-dir $BASEDIR/../artifacts/token \
  --contract-name @bindings/token \
  --contract-id "" &>/dev/null

(cd $BASEDIR/../artifacts; rm -r debttoken/node_modules stoken/node_modules pool/node_modules token/node_modules 1>/dev/null)

echo "Bindings have been generated"

(cd $BASEDIR/../artifacts; zip -r contract-bindings.zip debttoken stoken pool token 1>/dev/null)
(cd $BASEDIR/../artifacts; rm -r debttoken stoken pool token 1>/dev/null)

echo "Bindings archive have been created"
