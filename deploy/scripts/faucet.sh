# xrp

soroban contract invoke \
    --rpc-url "https://soroban-testnet.stellar.org/" \
    --network-passphrase "Test SDF Network ; September 2015"\
    --source SDLECWCIRM7DAZDUMMI4GKDDCPWQLEOITGDQXLPDWCL6R6F64XQPOTNK \
    --id CA2BPDJPXQP5B75LLIQ6HBNEFEBSL266FKBTO4V7GBRXFLL2QXMPSCN4 \
    -- \
    mint \
    --to GDLJFYGSJS6STVIWDVXT2GOM3BTNWGSMWHQF3CEIP633SOAS7KH3Q5N5 \
    --amount 1000000000000

# usdc

soroban contract invoke \
    --rpc-url "https://soroban-testnet.stellar.org/" \
    --network-passphrase "Test SDF Network ; September 2015"\
    --source SDLECWCIRM7DAZDUMMI4GKDDCPWQLEOITGDQXLPDWCL6R6F64XQPOTNK \
    --id CDEH5MIGB6K6ZOOIZKQ66FRCND54RVG7GBMAVX3XKLWSCADYOJBCXQKN \
    -- \
    mint \
    --to GDLJFYGSJS6STVIWDVXT2GOM3BTNWGSMWHQF3CEIP633SOAS7KH3Q5N5 \
    --amount 1000000000000