#!/bin/bash

BASEDIR=$(dirname $0)
ARTIFACTS=$BASEDIR/../artifacts

source $ARTIFACTS/.contracts

soroban contract bindings typescript \
  --wasm $ARTIFACTS/debt_token.wasm \
  --output-dir $ARTIFACTS/debttoken \
  --contract-id "" &>/dev/null

soroban contract bindings typescript \
  --wasm $ARTIFACTS/s_token.wasm \
  --output-dir $ARTIFACTS/stoken \
  --contract-id "" &>/dev/null

soroban contract bindings typescript \
  --wasm $ARTIFACTS/pool.wasm \
  --output-dir $ARTIFACTS/pool \
  --contract-id $SLENDER_POOL &>/dev/null

soroban contract bindings typescript \
  --wasm $ARTIFACTS/token.wasm \
  --output-dir $ARTIFACTS/token \
  --contract-id "" &>/dev/null

(cd $ARTIFACTS; rm -r debttoken/node_modules stoken/node_modules pool/node_modules token/node_modules 1>/dev/null)

echo "Bindings have been generated"

(cd $ARTIFACTS; zip -r contract-bindings.zip debttoken stoken pool token 1>/dev/null)
(cd $ARTIFACTS; rm -r debttoken stoken pool token 1>/dev/null)

echo "Bindings archive have been created"
