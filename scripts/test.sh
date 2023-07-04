#!/bin/bash

declare $(cat scripts/.env)
declare $(cat scripts/.contracts)

invoke() {
    local result=$(soroban contract invoke \
        --id $1 \
        --source $2 \
        --rpc-url http://localhost:8000/soroban/rpc \
        --network-passphrase 'Standalone Network ; February 2017' \
        -- \
        $3)
    echo $result
}

readBalance() {
    echo $(invoke $1 $2 "balance --id $3")
}

print() {
    echo "User token balance $(readBalance $TOKEN $USER_SECRET $USER_PUBLIC)"
    echo "User stoken balance $(readBalance $STOKEN $USER_SECRET $USER_PUBLIC)"
    echo "Pool token balance $(readBalance $TOKEN $USER_SECRET $STOKEN)"
}

echo "User: $USER_PUBLIC"
print

DEPOSIT=3000000000
WITHDRAW=1500000000

echo -e "\nDeposit $DEPOSIT tokens"
invoke $POOL $USER_SECRET "deposit \
    --who $USER_PUBLIC \
    --asset $TOKEN \
    --amount $DEPOSIT" 1>/dev/null
print

echo -e "\nWithdraw $WITHDRAW tokens"
invoke $POOL $USER_SECRET "withdraw \
    --who $USER_PUBLIC \
    --asset $TOKEN \
    --amount $WITHDRAW
    --to $USER_PUBLIC" 1>/dev/null
print
