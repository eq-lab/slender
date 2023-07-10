#!/bin/bash

#Read $TOKEN, $TOKEN_SECRET, $USER_PUBLIC, $POOL, $POOL_SECRET, $STOKEN, $DEBT_TOKEN, $PRICE_FEED

source scripts/.env
source scripts/.contracts

curl -s "$FRIENDBOT_URL?addr=$USER_PUBLIC" 1>/dev/null

# Mint amount $MINT_AMOUNT to user $USER_PUBLIC
soroban contract invoke \
    --id $TOKEN \
    --source $TOKEN_SECRET \
    --rpc-url http://localhost:8000/soroban/rpc \
    --network-passphrase "$PASSPHRASE" \
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
    --network-passphrase "$PASSPHRASE" \
    -- \
    init_reserve \
    --asset $TOKEN \
    --input '{"s_token_address": "'$STOKEN'", "debt_token_address": "'$DEBT_TOKEN'"}' 1>/dev/null

echo "Pool reserve initialized"

# Initialize price feed oracle
soroban contract invoke \
    --id $POOL \
    --source $POOL_SECRET \
    --rpc-url http://localhost:8000/soroban/rpc \
    --network-passphrase "$PASSPHRASE" \
    -- \
    set_price_feed \
    --feed $PRICE_FEED \
    --assets '["GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN"]' 1>/dev/null

echo "Price feed is set"
