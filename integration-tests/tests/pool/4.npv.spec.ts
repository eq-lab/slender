import { SorobanClient, delay } from "../soroban.client";
import {
    accountPosition,
    borrow,
    cleanSlenderEnvKeys,
    debtTokenBalanceOf,
    debtTokenTotalSupply,
    deploy,
    deposit,
    healthFactor,
    init,
    initPrice,
    mintUnderlyingTo,
    sTokenBalanceOf,
    sTokenTotalSupply,
    sTokenUnderlyingBalanceOf,
    tokenBalanceOf,
    withdraw
} from "../pool.sut";
import { borrower1Keys, contractsFilename, lender1Keys } from "../soroban.config";
import { assert, expect, use } from "chai";
import chaiAsPromised from 'chai-as-promised';
use(chaiAsPromised);

describe("LendingPool: Borrower position", function () {
    let client: SorobanClient;
    let lender1Address: string;
    let borrower1Address: string;

    before(async function () {
        client = new SorobanClient();

        await cleanSlenderEnvKeys();
        await deploy();
        await init(client);

        lender1Address = lender1Keys.publicKey();
        borrower1Address = borrower1Keys.publicKey();

        // uncomment to resume test with existing contracts
        // require("dotenv").config({ path: contractsFilename });
        // return;

        await Promise.all([
            client.registerAccount(lender1Address),
            client.registerAccount(borrower1Address),
        ]);

        await mintUnderlyingTo(client, "XLM", lender1Address, 1_000_000_000n);
        await mintUnderlyingTo(client, "XRP", borrower1Address, 100_000_000_000n);
    });

    it("Case 1: Lender & Borrower make deposits", async function () {
        // Lender1 deposits 50 XLM
        await deposit(client, lender1Keys, "XLM", 500_000_000n);

        // Borrower1 deposits 20 XRP
        await deposit(client, borrower1Keys, "XRP", 20_000_000_000n);

        const lender1XlmBalance = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalance = await sTokenBalanceOf(client, "XLM", lender1Address);

        const borrower1XrpBalance = await tokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SXrpBalance = await sTokenBalanceOf(client, "XRP", borrower1Address);

        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");

        const sXlmSupply = await sTokenTotalSupply(client, "XLM");
        const sXrpSupply = await sTokenTotalSupply(client, "XRP");

        assert.equal(lender1XlmBalance, 500_000_000n);
        assert.equal(lender1SXlmBalance, 500_000_000n);

        assert.equal(borrower1XrpBalance, 80_000_000_000n);
        assert.equal(borrower1SXrpBalance, 20_000_000_000n);

        assert.equal(sXlmBalance, 500_000_000n);
        assert.equal(sXrpBalance, 20_000_000_000n);

        assert.equal(sXlmSupply, 500_000_000n);
        assert.equal(sXrpSupply, 20_000_000_000n);
    });

    it("Case 2: Borrower borrows with NPV > 0", async function () {
        await delay(20_000);

        // Borrower1 borrows 5 XLM
        await borrow(client, borrower1Keys, "XLM", 50_000_000n);

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XlmBalance, 50_000_000n);
        assert.equal(borrower1DXlmBalance, 50_000_001n);
        assert.equal(sXlmBalance, 450_000_000n);
        assert.equal(dXlmSupply, 50_000_001n);

        assert(borrower1Position.debt >= 50_000_000n
            && borrower1Position.debt < 50_010_000n);
        assert.equal(borrower1Position.discounted_collateral, 120_000_000n);
        assert(borrower1Position.npv > 69_990_000n
            && borrower1Position.npv <= 70_000_000n, `borrower1Position.npv ${borrower1Position.npv}`);
    });

    it("Case 3: Borrower borrows more with health > 0.25", async function () {
        // Borrower1 borrows 3.9 XLM
        await borrow(client, borrower1Keys, "XLM", 39_000_000n);

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XlmBalance, 89_000_000n);
        assert(borrower1DXlmBalance >= 88_999_999n
            && borrower1DXlmBalance < 100_000_000n, `borrower1DXlmBalance ${borrower1DXlmBalance} `);
        assert.equal(sXlmBalance, 411_000_000n);
        assert.equal(dXlmSupply, borrower1DXlmBalance);

        assert(borrower1Position.debt >= 89_000_000n
            && borrower1Position.debt < 89_000_100n, `borrower1Position.debt ${borrower1Position.debt}`);
        assert.equal(borrower1Position.discounted_collateral, 120_000_000n);
        assert(borrower1Position.npv > 30_999_900
            && borrower1Position.npv <= 31_000_000n, `borrower1Position.npv ${borrower1Position.npv}`);
    });

    it("Case 4: Borrower withdraws with npv > 0", async function () {
        // Borrower1 withdraws 5.0 XRP
        const borrower1PositionBefore = await accountPosition(client, borrower1Keys);
        console.log(borrower1PositionBefore);

        await withdraw(client, borrower1Keys, "XRP", 5_000_000_000n);

        const borrower1XrpBalance = await tokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SXrpBalance = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");
        const sXrpSupply = await sTokenTotalSupply(client, "XRP");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XrpBalance, 85_000_000_000n);
        assert.equal(borrower1SXrpBalance, 15_000_000_000n);
        assert.equal(sXrpBalance, 15_000_000_000n);
        assert.equal(sXrpSupply, borrower1SXrpBalance);

        assert(borrower1Position.debt >= 89_000_000n
            && borrower1Position.debt < 89_000_100n, `borrower1Position.debt ${borrower1Position.debt}`);
        assert.equal(borrower1Position.discounted_collateral, 90_000_000n);
        assert(borrower1Position.npv > 999_000n
            && borrower1Position.npv <= 1_000_000n, `borrower1Position.npv ${borrower1Position.npv}`);
    });

    it("Case 5: Drop the XRP price so Borrower's NPV <= 0", async function () {
        // XRP price is set to 0.9
        await initPrice(client, "XRP", 9_000_000_000_000_000n, 0);

        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert(borrower1Position.npv < 0n
            && borrower1Position.npv > -10_000_000n);
    });

    it("Case 6: Borrower tries to borrow", async function () {
        // Borrower1 borrows 0.0001 XLM
        await expect(borrow(client, borrower1Keys, "XLM", 1_000n)).to.eventually.rejected;

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XlmBalance, 89_000_000n);
        assert(borrower1DXlmBalance > 88_000_000n
            && borrower1DXlmBalance < 89_000_002n, `borrower1DXlmBalance: ${borrower1DXlmBalance}`);
        assert.equal(sXlmBalance, 411_000_000n);
        assert.equal(dXlmSupply, borrower1DXlmBalance);

        assert(borrower1Position.debt >= 89_000_000n
            && borrower1Position.debt < 90_000_000n, `borrower1Position.debt ${borrower1Position.debt}`);
        assert.equal(borrower1Position.discounted_collateral, 81_000_000n);
        assert(borrower1Position.npv < 0n
            && borrower1Position.npv > -10_000_000n);
    });

    it("Case 7: Borrower tries to withdraw", async function () {
        // Borrower1 withdraws 0.000001 XRP
        await expect(withdraw(client, borrower1Keys, "XRP", 1_000n)).to.eventually.rejected;

        const borrower1XrpBalance = await tokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SXrpBalance = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");
        const sXrpSupply = await sTokenTotalSupply(client, "XRP");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XrpBalance, 85_000_000_000n);
        assert.equal(borrower1SXrpBalance, 15_000_000_000n);
        assert.equal(sXrpBalance, 15_000_000_000n);
        assert.equal(sXrpSupply, borrower1SXrpBalance);

        assert(borrower1Position.debt >= 89_000_000n
            && borrower1Position.debt < 90_000_000n, `borrower1Position.debt ${borrower1Position.debt}`);
        assert.equal(borrower1Position.discounted_collateral, 81_000_000n);
        assert(borrower1Position.npv < 0n
            && borrower1Position.npv > -10_000_000n);
    });

    it("Case 8: Borrower deposits more to achieve good NPV and health < 0.25", async function () {
        // Borrower1 deposits 3.5 XRP
        await deposit(client, borrower1Keys, "XRP", 3_500_000_000n);

        const borrower1XrpBalance = await tokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SXrpBalance = await sTokenBalanceOf(client, "XRP", borrower1Address);

        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");

        const sXrpSupply = await sTokenTotalSupply(client, "XRP");

        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XrpBalance, 81_500_000_000n);
        assert.equal(borrower1SXrpBalance, 18_500_000_000n);

        assert.equal(sXrpBalance, 18_500_000_000n);

        assert.equal(sXrpSupply, 18_500_000_000n);

        assert(borrower1Position.debt >= 89_000_000n
            && borrower1Position.debt < 90_000_000n, `borrower1Position.debt ${borrower1Position.debt}`);
        assert.equal(borrower1Position.discounted_collateral, 99_900_000n);
        assert(borrower1Position.npv < 11_000_000n
            && borrower1Position.npv > 10_000_000n);
        assert(healthFactor(borrower1Position) < 0.25);
    });

    it("Case 9: Borrower tries to borrow more with NPV > 0 and health < 0.25", async function () {
        // Borrower1 borrows 0.5 XLM
        await expect(borrow(client, borrower1Keys, "XLM", 5_000_000n)).to.eventually.rejected;

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XlmBalance, 89_000_000n);
        assert(borrower1DXlmBalance > 88_000_000n
            && borrower1DXlmBalance < 89_000_002n, `borrower1DXlmBalance: ${borrower1DXlmBalance}`);
        assert.equal(sXlmBalance, 411_000_000n);
        assert.equal(dXlmSupply, borrower1DXlmBalance);

        assert(borrower1Position.debt >= 89_000_000n
            && borrower1Position.debt < 90_000_000n, `borrower1Position.debt ${borrower1Position.debt}`);
        assert.equal(borrower1Position.discounted_collateral, 99_900_000n);
        assert(borrower1Position.npv < 11_000_000n
            && borrower1Position.npv > 10_000_000n);
        assert(healthFactor(borrower1Position) < 0.25);
    });

    it("Case 10: Borrower deposits more to achieve health >= 0.25", async function () {
        // Borrower1 deposits 3.5 XRP
        await deposit(client, borrower1Keys, "XRP", 3_500_000_000n);

        const borrower1XrpBalance = await tokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SXrpBalance = await sTokenBalanceOf(client, "XRP", borrower1Address);

        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");

        const sXrpSupply = await sTokenTotalSupply(client, "XRP");

        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XrpBalance, 78_000_000_000n);
        assert.equal(borrower1SXrpBalance, 22_000_000_000n);

        assert.equal(sXrpBalance, 22_000_000_000n);

        assert.equal(sXrpSupply, 22_000_000_000n);

        assert(borrower1Position.debt >= 89_000_000n
            && borrower1Position.debt < 90_000_000n, `borrower1Position.debt ${borrower1Position.debt}`);
        assert.equal(borrower1Position.discounted_collateral, 118_800_000n);
        assert(borrower1Position.npv < 30_000_000n
            && borrower1Position.npv > 29_000_000n);
        assert(healthFactor(borrower1Position) >= 0.25);
    });

    it("Case 11: Borrower borrows more with health >= 0.25", async function () {
        // Borrower1 borrows 0.009 XLM
        await borrow(client, borrower1Keys, "XLM", 90_000n);

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XlmBalance, 89_090_000n);
        assert(borrower1DXlmBalance > 89_000_000n
            && borrower1DXlmBalance < 90_000_001n, `borrower1DXlmBalance: ${borrower1DXlmBalance}`);
        assert.equal(sXlmBalance, 410_910_000n);
        assert.equal(dXlmSupply, borrower1DXlmBalance);

        assert(borrower1Position.debt >= 89_000_000n
            && borrower1Position.debt < 90_000_000n, `borrower1Position.debt ${borrower1Position.debt}`);
        assert.equal(borrower1Position.discounted_collateral, 118_800_000n);
        assert(borrower1Position.npv < 30_000_000n
            && borrower1Position.npv > 29_000_000n);
        assert(healthFactor(borrower1Position) > 0.25);
    });
});