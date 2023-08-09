import { SorobanClient } from "../soroban.client";
import { expect } from "chai";
import { addressToScVal, arrayToScVal, parseScVal, i128ToScVal, objectToScVal, numberToScvU32, stringToScvString, stringToScvBytes } from "../soroban.converter";
import { borrower1Keys, adminKeys } from "../soroban.config";

describe("LendingPool", function () {
    let client: SorobanClient;

    before(async function () {
        client = new SorobanClient();

        await client.trySendTransaction(process.env.TOKEN, "initialize", adminKeys,
            addressToScVal(adminKeys.publicKey()),
            numberToScvU32(9),
            stringToScvString("Token"),
            stringToScvString("TKN"));

        stringToScvBytes("0000000000000000000000000000000000000000000000000000000000000000")
        stringToScvBytes(process.env.POOL_HASH)
        addressToScVal(adminKeys.publicKey())
        addressToScVal()
        // # deployPoolResult=$(invoke $deployer $POOL_SECRET "deploy_pool \
        // #     --salt 0000000000000000000000000000000000000000000000000000000000000000 \
        // #     --wasm_hash $poolHash \
        // #     --admin $POOL_PUBLIC")
        // # POOL=$(addressFromResult $deployPoolResult)
        await client.trySendTransaction(process.env.TOKEN, "deploy_pool", adminKeys,
            addressToScVal(adminKeys.publicKey()),
            numberToScvU32(9),
            stringToScvString("Token"),
            stringToScvString("TKN"));

        this.borrower = await client.registerAccount(borrower1Keys.publicKey());
    });

    it("should mint 10,000,000,000 TOKENs to USER", async function () {
        //     const debtTokenResult = await this.client.sendTransaction(
        //         process.env.TOKEN,
        //         "mint",
        //         tokenKeys,
        //         addressToScVal(process.env.USER_PUBLIC),
        //         i128ToScVal(BigInt(10000000000n))
        //     );

        //     expect(debtTokenResult.status).to.equal("SUCCESS");

        //     const userBalanceResult = await this.client.sendTransaction(
        //         process.env.TOKEN,
        //         "balance",
        //         tokenKeys,
        //         addressToScVal(process.env.USER_PUBLIC)
        //     );

        //     const minted = parseScVal(userBalanceResult.resultXdr);

        //     expect(userBalanceResult.status).to.equal("SUCCESS");
        //     expect(minted).to.equal(10000000000n);
    });

    // it("should initialize reserve and set S_TOKEN and DEBT_TOKEN", async function () {
    //     const initResult = await this.client.sendTransaction(
    //         process.env.POOL,
    //         "init_reserve",
    //         poolKeys,
    //         addressToScVal(process.env.TOKEN),
    //         objectToScVal({
    //             "debt_token_address": addressToScVal(process.env.DEBT_TOKEN),
    //             "s_token_address": addressToScVal(process.env.STOKEN)
    //         })
    //     );

    //     expect(initResult.status).to.equal("SUCCESS");

    //     const reserveResult = await this.client.sendTransaction(
    //         process.env.POOL,
    //         "get_reserve",
    //         poolKeys,
    //         addressToScVal(process.env.TOKEN)
    //     );

    //     const value: any = parseScVal(reserveResult.resultXdr);

    //     expect(reserveResult.status).to.equal("SUCCESS");
    //     expect(value.debt_token_address).to.equal(process.env.DEBT_TOKEN);
    //     expect(value.s_token_address).to.equal(process.env.STOKEN);
    // });

    // it("should set PRICE_FEED", async function () {
    //     const setPriceFeedResult = await this.client.sendTransaction(
    //         process.env.POOL,
    //         "set_price_feed",
    //         poolKeys,
    //         addressToScVal(process.env.PRICE_FEED),
    //         arrayToScVal([
    //             addressToScVal("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN")
    //         ])
    //     );

    //     expect(setPriceFeedResult.status).to.equal("SUCCESS");

    //     const priceFeedResult = await this.client.sendTransaction(
    //         process.env.POOL,
    //         "get_price_feed",
    //         poolKeys,
    //         addressToScVal("GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN")
    //     );

    //     const value = parseScVal(priceFeedResult.resultXdr);

    //     expect(priceFeedResult.status).to.equal("SUCCESS");
    //     expect(value).to.equal(process.env.PRICE_FEED);
    // });

    // it("should deposit reserve 3,000,000,000 to USER balance", async function () {
    //     const depositResult = await this.client.sendTransaction(
    //         process.env.POOL,
    //         "deposit",
    //         userKeys,
    //         addressToScVal(process.env.USER_PUBLIC),
    //         addressToScVal(process.env.TOKEN),
    //         i128ToScVal(BigInt(3000000000n))
    //     );

    //     expect(depositResult.status).to.equal("SUCCESS");

    //     const balanceResult = await this.client.sendTransaction(
    //         process.env.TOKEN,
    //         "balance",
    //         userKeys,
    //         addressToScVal(process.env.USER_PUBLIC),
    //     );

    //     const balance = parseScVal(balanceResult.resultXdr);

    //     expect(balance).to.equal(7000000000n);
    // });

    // it("should withdraw 1,500,000,000 from USER balance", async function () {
    //     const withdrawResult = await this.client.sendTransaction(
    //         process.env.POOL,
    //         "withdraw",
    //         userKeys,
    //         addressToScVal(process.env.USER_PUBLIC),
    //         addressToScVal(process.env.TOKEN),
    //         i128ToScVal(BigInt(1500000000n)),
    //         addressToScVal(process.env.USER_PUBLIC)
    //     );

    //     expect(withdrawResult.status).to.equal("SUCCESS");

    //     const balanceResult = await this.client.sendTransaction(
    //         process.env.STOKEN,
    //         "balance",
    //         userKeys,
    //         addressToScVal(process.env.USER_PUBLIC),
    //     );

    //     const balance = parseScVal(balanceResult.resultXdr);

    //     expect(balance).to.equal(1500000000n);
    // });
});
