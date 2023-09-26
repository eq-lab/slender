import { Keypair, xdr } from "soroban-client";
import { I128_MAX, accountPosition, borrow, cleanSlenderEnvKeys, deploy, deposit, init, liquidate, liquidateCli, mintUnderlyingTo, repay, setPrice, withdraw, writeBudgetSnapshot } from "../pool.sut";
import { SorobanClient, delay } from "../soroban.client";
import { adminKeys, lender1Keys } from "../soroban.config";
import { assert } from "chai";
import { parseScvToJs } from "../soroban.converter";
import * as fs from 'fs';

const CASE_1_LOG = "snapshots/liquidateUnderlying1.log";
const CASE_2_LOG = "snapshots/liquidateUnderlying2.log";
const CASE_3_LOG = "snapshots/liquidateUnderlying3.log";
const CASE_4_LOG = "snapshots/liquidateStoken4.log";
const CASE_5_LOG = "snapshots/liquidateStoken5.log";
const CASE_6_LOG = "snapshots/liquidateStoken6.log";
const CASE_7_LOG = "snapshots/liquidateStoken7.log";
const CASE_8_LOG = "snapshots/liquidateStoken8.log";
const CASE_9_LOG = "snapshots/liquidateStoken9.log";

function tryRemoveLogFile(name: string) {
    try {
        fs.unlinkSync(name);
    } catch (e) {
        if (e.code !== "ENOENT") {
            throw e;
        }
    }
}

describe("LendingPool: liquidation cases must not exceed CPU/MEM limits", function () {
    let client: SorobanClient;
    let lender1Address: string;
    let borrower1Keys: Keypair;
    let borrower2Keys: Keypair;
    let liquidatorKeys: Keypair;
    let borrower1Address: string;
    let borrower2Address: string;
    let liquidatorAddress: string;
    let xrpPrice: bigint = 1_000_000_000n;
    let usdcPrice: bigint = 1_000_000_000n;

    before(async function () {
        client = new SorobanClient();

        await cleanSlenderEnvKeys();
        await deploy();
        await init(client);

        lender1Address = lender1Keys.publicKey();

        await client.registerAccount(lender1Address);

        await client.sendTransaction(
            process.env.SLENDER_POOL,
            "set_reserve_timestamp_window",
            adminKeys,
            3,
            xdr.ScVal.scvU64(xdr.Uint64.fromString(BigInt(100).toString()))
        );

        let res = await client.simulateTransaction(process.env.SLENDER_POOL, "reserve_timestamp_window");
        assert.equal(parseScvToJs(res) as number, 100);
        console.log("reserve_timestamp_window", parseScvToJs(res));
        
        await mintUnderlyingTo(client, "XLM", lender1Address, 400_000_000_000n);
        await mintUnderlyingTo(client, "XRP", lender1Address, 400_000_000_000n);
        await mintUnderlyingTo(client, "USDC", lender1Address, 400_000_000_000n);

        // Lender1 deposits XLM, XRP, USDC
        await deposit(client, lender1Keys, "XLM", 160_000_000_000n);
        await deposit(client, lender1Keys, "XRP", 160_000_000_000n);
        await deposit(client, lender1Keys, "USDC", 160_000_000_000n);

        await delay(100_000);

        for (const name of [CASE_1_LOG, CASE_2_LOG, CASE_3_LOG, CASE_4_LOG, CASE_5_LOG, CASE_6_LOG, CASE_7_LOG, CASE_7_LOG, CASE_8_LOG, CASE_9_LOG]) {
            tryRemoveLogFile(name);
        }
    })

    beforeEach(async function () {
        // liquidator with 2 debts
        liquidatorKeys = Keypair.random();
        liquidatorAddress = liquidatorKeys.publicKey();
        // borrower with 1 debt
        borrower1Keys = Keypair.random();
        borrower1Address = borrower1Keys.publicKey();
        // borrower with 2 debts
        borrower2Keys = Keypair.random();
        borrower2Address = borrower2Keys.publicKey();

        await Promise.all([
            client.registerAccount(liquidatorAddress),
            client.registerAccount(borrower1Address),
            client.registerAccount(borrower2Address),
        ]);

        for (const address of [liquidatorAddress, borrower1Address, borrower2Address]) {
            await mintUnderlyingTo(client, "XLM", address, 100_000_000_000n);
            await mintUnderlyingTo(client, "XRP", address, 100_000_000_000n);
            await mintUnderlyingTo(client, "USDC", address, 100_000_000_000n);
        }

        await deposit(client, liquidatorKeys, "USDC", (10_000_000_000n * 1_000_000_000n / usdcPrice));
        await borrow(client, liquidatorKeys, "XLM", 1_000_000_000n);
        await borrow(client, liquidatorKeys, "XRP", (1_000_000_000n * 1_000_000_000n / xrpPrice));

        // Borrower1 deposits 10_000_000_000 XLM, XRP, borrows 19_000_000_000 USDC
        await deposit(client, borrower1Keys, "XLM", 10_000_000_000n);
        await deposit(client, borrower1Keys, "XRP", (30_000_000_000n * 1_000_000_000n / xrpPrice));
        await borrow(client, borrower1Keys, "USDC", (19_000_000_000n * 1_000_000_000n / usdcPrice));

        // Borrower2 deposits 20_000_000_000 USDC, borrows 6_000_000_000 XLM, 5_999_000_000 XRP
        await deposit(client, borrower2Keys, "USDC", (20_000_000_000n * 1_000_000_000n / usdcPrice));
        await borrow(client, borrower2Keys, "XLM", 6_000_000_000n);
        await borrow(client, borrower2Keys, "XRP", (5_900_000_000n * 1_000_000_000n / xrpPrice));
    })

    it("Case 1: liquidate with receiving underlying when borrower has one debt and two deposits", async function () {
        console.log(await accountPosition(client, borrower1Keys));
        
        usdcPrice = usdcPrice * 1_500_000_000n / 1_000_000_000n;
        await setPrice(client, "USDC", usdcPrice);

        console.log(await accountPosition(client, borrower1Keys));

        try {
            await liquidate(client, liquidatorKeys, borrower1Address, false)
                .then((result) => writeBudgetSnapshot("liquidateUnderlying1", result));
        } catch (e) {
            console.error(e);
            const liquidateRes = await liquidateCli(liquidatorKeys, borrower1Address, false);
            fs.writeFileSync(CASE_1_LOG, liquidateRes);
        }
    })

    it("Case 2: liquidate with receiving underlying when borrower has one debt and one deposit", async function () {
        await deposit(client, borrower1Keys, "XRP", 10_000_000_000n * 1_000_000_000n / xrpPrice);
        await withdraw(client, borrower1Keys, "XLM", I128_MAX);
        
        console.log(await accountPosition(client, borrower1Keys));
        
        usdcPrice = usdcPrice * 1_500_000_000n / 1_000_000_000n;
        await setPrice(client, "USDC", usdcPrice);

        console.log(await accountPosition(client, borrower1Keys));

        try {
            await liquidate(client, liquidatorKeys, borrower1Address, false)
                .then((result) => writeBudgetSnapshot("liquidateUnderlying2", result));
        } catch (e) {
            console.error(e);
            const liquidateRes = await liquidateCli(liquidatorKeys, borrower1Address, false);
            fs.writeFileSync(CASE_2_LOG, liquidateRes);
        }
    })

    it("Case 3: liquidate with receiving underlying when borrower has two debts and one deposit", async function () {
        console.log(await accountPosition(client, borrower2Keys));
        
        xrpPrice = xrpPrice * 1_500_000_000n / 1_000_000_000n;
        await setPrice(client, "XRP", xrpPrice);

        console.log(await accountPosition(client, borrower2Keys));

        try {
            await liquidate(client, liquidatorKeys, borrower2Address, false)
                .then((result) => writeBudgetSnapshot("liquidateUnderlying3", result));
        } catch (e) {
            console.error(e);
            const liquidateRes = await liquidateCli(liquidatorKeys, borrower2Address, false);
            fs.writeFileSync(CASE_3_LOG, liquidateRes);
        }
    })

    it("Case 4: liquidate with receiving sToken & repay when borrower has one debt and two deposits", async function () {
        console.log(await accountPosition(client, borrower1Keys));
        
        usdcPrice = usdcPrice * 1_500_000_000n / 1_000_000_000n;
        await setPrice(client, "USDC", usdcPrice);

        console.log(await accountPosition(client, borrower1Keys));

        try {
            await liquidate(client, liquidatorKeys, borrower1Address, true)
                .then((result) => writeBudgetSnapshot("liquidateSToken4", result));
        } catch (e) {
            console.error(e);
            const liquidateRes = await liquidateCli(liquidatorKeys, borrower1Address, true);
            fs.writeFileSync(CASE_4_LOG, liquidateRes);
        }
    })

    it("Case 5: liquidate with receiving sToken & repay when borrower has one debt and one deposit", async function () {
        console.log(await accountPosition(client, borrower1Keys));
        await deposit(client, borrower1Keys, "XRP", 10_000_000_000n * 1_000_000_000n / xrpPrice);
        await withdraw(client, borrower1Keys, "XLM", I128_MAX);
     
        console.log(await accountPosition(client, borrower1Keys));
        
        usdcPrice = usdcPrice * 1_500_000_000n / 1_000_000_000n;
        await setPrice(client, "USDC", usdcPrice);

        console.log(await accountPosition(client, borrower1Keys));

        try {
            await liquidate(client, liquidatorKeys, borrower1Address, true)
                .then((result) => writeBudgetSnapshot("liquidateSToken5", result));
        } catch (e) {
            console.error(e);
            const liquidateRes = await liquidateCli(liquidatorKeys, borrower1Address, true);
            fs.writeFileSync(CASE_5_LOG, liquidateRes);
        }
    })

    it("Case 6: liquidate with receiving sToken & without repay when borrower has one debt and two deposits", async function () {
        await repay(client, liquidatorKeys, "XLM", I128_MAX);
        await repay(client, liquidatorKeys, "XRP", I128_MAX);
        
        console.log(await accountPosition(client, borrower1Keys));
        
        usdcPrice = usdcPrice * 1_500_000_000n / 1_000_000_000n;
        await setPrice(client, "USDC", usdcPrice);

        console.log(await accountPosition(client, borrower1Keys));

        try {
            await liquidate(client, liquidatorKeys, borrower1Address, true)
                .then((result) => writeBudgetSnapshot("liquidateSToken6", result));
        } catch (e) {
            console.error(e);
            const liquidateRes = await liquidateCli(liquidatorKeys, borrower1Address, true);
            fs.writeFileSync(CASE_6_LOG, liquidateRes);
        }
    })

    it("Case 7: liquidate with receiving sToken & without repay when borrower has one debt and one deposit", async function () {
        await deposit(client, borrower1Keys, "XRP", 10_000_000_000n * 1_000_000_000n / xrpPrice);
        await withdraw(client, borrower1Keys, "XLM", I128_MAX);
        await repay(client, liquidatorKeys, "XLM", I128_MAX);
        await repay(client, liquidatorKeys, "XRP", I128_MAX);
        
        console.log(await accountPosition(client, borrower1Keys));
        
        usdcPrice = usdcPrice * 1_500_000_000n / 1_000_000_000n;
        await setPrice(client, "USDC", usdcPrice);

        console.log(await accountPosition(client, borrower1Keys));

        try {
            await liquidate(client, liquidatorKeys, borrower1Address, true)
                .then((result) => writeBudgetSnapshot("liquidateSToken7", result));
        } catch (e) {
            console.error(e);
            const liquidateRes = await liquidateCli(liquidatorKeys, borrower1Address, true);
            fs.writeFileSync(CASE_7_LOG, liquidateRes);
        }
    })

    it("Case 8: liquidate with receiving sToken & repay when borrower has two debts and one deposit", async function () {
        console.log(await accountPosition(client, borrower2Keys));
        
        xrpPrice = xrpPrice * 1_500_000_000n / 1_000_000_000n;
        await setPrice(client, "XRP", xrpPrice);

        console.log(await accountPosition(client, borrower2Keys));

        try {
            await liquidate(client, liquidatorKeys, borrower2Address, true)
                .then((result) => writeBudgetSnapshot("liquidateSToken8", result));
        } catch (e) {
            console.error(e);
            const liquidateRes = await liquidateCli(liquidatorKeys, borrower2Address, true);
            fs.writeFileSync(CASE_8_LOG, liquidateRes);
        }
    })

    it("Case 9: liquidate with receiving sToken & without repay when borrower has two debts and one deposit", async function () {
        await repay(client, liquidatorKeys, "XLM", I128_MAX);
        await repay(client, liquidatorKeys, "XRP", I128_MAX);
        console.log(await accountPosition(client, borrower2Keys));
        
        xrpPrice = xrpPrice * 1_500_000_000n / 1_000_000_000n;
        await setPrice(client, "XRP", xrpPrice);

        console.log(await accountPosition(client, borrower2Keys));

        try {
            await liquidate(client, liquidatorKeys, borrower2Address, true)
                .then((result) => writeBudgetSnapshot("liquidateSToken9", result));
        } catch (e) {
            console.error(e);
            const liquidateRes = await liquidateCli(liquidatorKeys, borrower2Address, true);
            fs.writeFileSync(CASE_9_LOG, liquidateRes);
        }
    })
})