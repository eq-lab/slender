#!/bin/bash

BASEDIR=$(dirname $0)

if [ $1 == "mainnet" ]
then
    ARTIFACTS=$BASEDIR/../artifacts/mainnet
    source $ARTIFACTS/.contracts
    source $BASEDIR/.$1.env

    stellar contract bindings typescript \
      --output-dir $ARTIFACTS/debttoken \
      --rpc-url "$SOROBAN_RPC_URL" \
      --network-passphrase "$PASSPHRASE" \
      --contract-id $SLENDER_DEBT_TOKEN_XLM &>/dev/null

    stellar contract bindings typescript \
      --output-dir $ARTIFACTS/stoken \
      --rpc-url "$SOROBAN_RPC_URL" \
      --network-passphrase "$PASSPHRASE" \
      --contract-id $SLENDER_S_TOKEN_XLM &>/dev/null

    stellar contract bindings typescript \
      --output-dir $ARTIFACTS/pool \
      --rpc-url "$SOROBAN_RPC_URL" \
      --network-passphrase "$PASSPHRASE" \
      --contract-id $SLENDER_POOL &>/dev/null

    # bindings for the token contract is copy-pasted from testnet
else
    ARTIFACTS=$BASEDIR/../artifacts/testnet
    source $ARTIFACTS/.contracts
    source $BASEDIR/.$1.env

    stellar contract bindings typescript \
      --output-dir $ARTIFACTS/debttoken \
      --rpc-url "$SOROBAN_RPC_URL" \
      --network-passphrase "$PASSPHRASE" \
      --contract-id $SLENDER_DEBT_TOKEN_XLM &>/dev/null

    stellar contract bindings typescript \
      --output-dir $ARTIFACTS/stoken \
      --rpc-url "$SOROBAN_RPC_URL" \
      --network-passphrase "$PASSPHRASE" \
      --contract-id $SLENDER_S_TOKEN_XLM &>/dev/null

    stellar contract bindings typescript \
      --output-dir $ARTIFACTS/pool \
      --rpc-url "$SOROBAN_RPC_URL" \
      --network-passphrase "$PASSPHRASE" \
      --contract-id $SLENDER_POOL &>/dev/null

    stellar contract bindings typescript \
      --output-dir $ARTIFACTS/token \
      --rpc-url "$SOROBAN_RPC_URL" \
      --network-passphrase "$PASSPHRASE" \
      --contract-id $SLENDER_TOKEN_XRP &>/dev/null
fi

(cd $ARTIFACTS; rm -r debttoken/node_modules stoken/node_modules pool/node_modules token/node_modules 1>/dev/null)

echo "Bindings have been generated"

(cd $ARTIFACTS; zip -r contract-bindings.zip debttoken stoken pool token 1>/dev/null)
(cd $ARTIFACTS; rm -r debttoken stoken pool token 1>/dev/null)

echo "Bindings archive have been created"
