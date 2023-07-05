# Slender

The Slender lending protocol uses a pool-based strategy that aggregates each user's supplied assets. Users will be able to provide some assets to the protocol and earn interest.

When users supply liquidity they get LP tokens or as we call them sTokens in return. sTokens accrue interest and reflect this accrual in their “price”.

## Build

### Prerequisites

To build and run unit tests you need to install **rust**. See https://soroban.stellar.org/docs/getting-started/setup

For building all packages run:

```shell
make
```

## Test

In order to run unit tests use command below:

```shell
make test
```

## Deploy and run demo in a local network

### Prerequisites

To run demo script you need to install **soroban-cli** version 0.8.7

```shell
cargo install --locked --version 0.8.7 soroban-cli
```

Scenario:

1. Deploy pool, sToken, token and debt token
2. Initialize user with initial balance
3. Initialize token as a reserve token of a pool
4. Deposit some amount into a pool. Show balances of user and sToken
5. Withdraw some amount from a pool. Show balances of user and sToken

Build contracts:

```shell
make
```

Run a local standalone network with the following command:

```shell
docker run --rm -it \
  -p 8000:8000 \
  --name stellar \
  stellar/quickstart:soroban-dev@sha256:57e8ab498bfa14c65595fbb01cb94b1cdee9637ef2e6634e59d54f6958c05bdb \
  --standalone \
  --enable-soroban-rpc
```

Run simulation script (stay at root project directory):

```shell
./scripts/demo.sh
```

All parameters of demo script could be found in `scripts/.env`

### Futurenet

To deploy and run demo script in the Futurenet network you should run soroban-rpc with command

```shell
docker run --rm -it \
   -p 8000:8000 \
   --name stellar \
   stellar/quickstart:soroban-dev@sha256:57e8ab498bfa14c65595fbb01cb94b1cdee9637ef2e6634e59d54f6958c05bdb \
   --futurenet \
   --enable-soroban-rpc
```

copy all variables from `.futurenet.env` to `.env` file and run `./scripts/demo.sh`

### How to add token to freighter wallet

In order to add new token to the freighter wallet you need to convert token address base32 representation to hex and use it as Token Id.

Script example:

```js
const { StrKey } = require("soroban-client");

let id = "CCLZ4QF5QSWBANABDZPC3XKHMVX3GUSLFLP4JS22SCIGLADD2E7STHDR";
console.log(StrKey.decodeContract(id).toString("hex"));
//result
//979e40bd84ac1034011e5e2ddd47656fb3524b2adfc4cb5a9090658063d13f29
```
