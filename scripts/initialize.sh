#!/bin/bash

#Read $TOKEN, $TOKEN_SECRET, $USER_PUBLIC, $POOL, $POOL_SECRET, $STOKEN, $DEBT_TOKEN
declare $(cat scripts/.env)
declare $(cat scripts/.contracts)

curl -s "http://localhost:8000/friendbot?addr=$USER_PUBLIC" 1>/dev/null

# Mint amount $MINT_AMOUNT to user $USER_PUBLIC
soroban contract invoke \
    --id $TOKEN \
    --source $TOKEN_SECRET \
    --rpc-url http://localhost:8000/soroban/rpc \
    --network-passphrase 'Standalone Network ; February 2017' \
    -- \
    mint \
    --to $USER_PUBLIC \
    --amount $MINT_AMOUNT 1>/dev/null

echo "Amount $MINT_AMOUNT minted to address $USER_PUBLIC"

# Initialize reserve
soroban contract invoke \
    --id $POOL \
    --source $POOL_SECRET \
    --rpc-url http://localhost:8000/soroban/rpc \
    --network-passphrase 'Standalone Network ; February 2017' \
    -- \
    init_reserve \
    --asset $TOKEN \
    --input '{"s_token_address": "'$STOKEN'", "debt_token_address": "'$DEBT_TOKEN'"}' 1>/dev/null

echo "Pool reserve initialized"
