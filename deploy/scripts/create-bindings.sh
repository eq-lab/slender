#!/bin/bash

BASEDIR=$(dirname $0)
source $BASEDIR/../artifacts/.contracts

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
  --contract-id $SLENDER_POOL &>/dev/null

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
