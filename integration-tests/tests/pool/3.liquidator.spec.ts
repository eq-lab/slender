import { SorobanClient, delay } from "../soroban.client";
import {
    I128_MAX,
    accountPosition,
    borrow,
    cleanSlenderEnvKeys,
    debtTokenBalanceOf,
    debtTokenTotalSupply,
    deploy,
    deposit,
    init,
    initPrice,
    liquidate,
    mintUnderlyingTo,
    repay,
    sTokenBalanceOf,
    sTokenTotalSupply,
    sTokenUnderlyingBalanceOf,
    tokenBalanceOf
} from "../pool.sut";
import { borrower1Keys, lender1Keys, liquidator1Keys } from "../soroban.config";
import { assert, expect, use } from "chai";
import chaiAsPromised from 'chai-as-promised';
import { convertToScvAddress } from "../soroban.converter";
use(chaiAsPromised);

describe("LendingPool: Liquidation (receive STokens)", function () {
    let client: SorobanClient;
    let lender1Address: string;
    let borrower1Address: string;
    let liquidator1Address: string;

    before(async function () {
        client = new SorobanClient();

        await cleanSlenderEnvKeys();
        await deploy();
        await init(client);

        lender1Address = lender1Keys.publicKey();
        borrower1Address = borrower1Keys.publicKey();
        liquidator1Address = liquidator1Keys.publicKey();

        await Promise.all([
            client.registerAccount(lender1Address),
            client.registerAccount(borrower1Address),
            client.registerAccount(liquidator1Address),
        ]);

        await mintUnderlyingTo(client, "XLM", lender1Address, 1_000_000_000n);
        await mintUnderlyingTo(client, "XRP", lender1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "USDC", lender1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "XRP", borrower1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "USDC", borrower1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "XLM", liquidator1Address, 1_400_000_000n);
    });

    it("Case 1: Liquidator, Lender & Borrower deposit assets", async function () {
        // Lender1 deposits 300_000_000 XLM
        await deposit(client, lender1Keys, "XLM", 300_000_000n);
        // Lender1 deposits 30_000_000_000n XRP
        await deposit(client, lender1Keys, "XRP", 30_000_000_000n);
        // Lender1 deposits 30_000_000_000n USDC
        await deposit(client, lender1Keys, "USDC", 30_000_000_000n);

        // Borrower1 deposits 10_000_000_000 XRP
        await deposit(client, borrower1Keys, "XRP", 10_000_000_000n);
        // Borrower1 deposits 10_000_000_000 USDC
        await deposit(client, borrower1Keys, "USDC", 10_000_000_000n);

        // Liquidator1 deposits 400_000_000 XLM
        await deposit(client, liquidator1Keys, "XLM", 400_000_000n);

        const lender1XlmBalance = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalance = await sTokenBalanceOf(client, "XLM", lender1Address);
        const lender1XrpBalance = await tokenBalanceOf(client, "XRP", lender1Address);
        const lender1SXrpBalance = await sTokenBalanceOf(client, "XRP", lender1Address);
        const lender1UsdcBalance = await tokenBalanceOf(client, "USDC", lender1Address);
        const lender1SUsdcBalance = await sTokenBalanceOf(client, "USDC", lender1Address);

        const borrower1XrpBalance = await tokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SXrpBalance = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1UsdcBalance = await tokenBalanceOf(client, "USDC", borrower1Address);
        const borrower1SUsdcBalance = await sTokenBalanceOf(client, "USDC", borrower1Address);

        const liquidator1XlmBalance = await tokenBalanceOf(client, "XLM", liquidator1Address);
        const liquidator1SXlmBalance = await sTokenBalanceOf(client, "XLM", liquidator1Address);

        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");
        const sUsdcBalance = await sTokenUnderlyingBalanceOf(client, "USDC");

        const sXlmSupply = await sTokenTotalSupply(client, "XLM");
        const sXrpSupply = await sTokenTotalSupply(client, "XRP");
        const sUsdcSupply = await sTokenTotalSupply(client, "USDC");
        assert.equal(lender1XlmBalance, 700_000_000n);
        assert.equal(lender1SXlmBalance, 300_000_000n);
        assert.equal(lender1XrpBalance, 70_000_000_000n);
        assert.equal(lender1SXrpBalance, 30_000_000_000n);
        assert.equal(lender1UsdcBalance, 70_000_000_000n);
        assert.equal(lender1SUsdcBalance, 30_000_000_000n);

        assert.equal(borrower1XrpBalance, 90_000_000_000n);
        assert.equal(borrower1SXrpBalance, 10_000_000_000n);
        assert.equal(borrower1UsdcBalance, 90_000_000_000n);
        assert.equal(borrower1SUsdcBalance, 10_000_000_000n);

        assert.equal(liquidator1XlmBalance, 1_000_000_000n);
        assert.equal(liquidator1SXlmBalance, 400_000_000n);

        assert.equal(sXlmBalance, 700_000_000n);
        assert.equal(sXrpBalance, 40_000_000_000n);
        assert.equal(sUsdcBalance, 40_000_000_000n);

        assert.equal(sXlmSupply, 700_000_000n);
        assert.equal(sXrpSupply, 40_000_000_000n);
        assert.equal(sUsdcSupply, 40_000_000_000n);
    });

    it("Case 2: Borrower borrows XLM with npv ~= 0", async function () {
        await delay(20_000);

        // Borrower1 borrows 119_990_000 XLM
        await borrow(client, borrower1Keys, "XLM", 119_990_000n);

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XlmBalance, 119_990_000n);
        assert.equal(borrower1DXlmBalance, 119_990_000n);
        assert.equal(sXlmBalance, 580_010_000n);
        assert.equal(dXlmSupply, 119_990_000n);

        assert(borrower1Position.debt > 119_990_000n
            && borrower1Position.debt < 120_000_000n);
        assert.equal(borrower1Position.discounted_collateral, 120_000_000n);
        assert(borrower1Position.npv > 0
            && borrower1Position.npv < 10_000n);
    });

    it("Case 3: Liquidator borrows XRP", async function () {
        // Liquidator1 borrows 11_000_000_000 XRP
        await borrow(client, liquidator1Keys, "XRP", 11_000_000_000n);

        const liquidator1XrpBalance = await tokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1DXrpBalance = await debtTokenBalanceOf(client, "XRP", liquidator1Address);
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");
        const dXrpSupply = await debtTokenTotalSupply(client, "XRP");
        const liquidator1Position = await accountPosition(client, liquidator1Keys);

        assert.equal(liquidator1XrpBalance, 11_000_000_000n);
        assert.equal(liquidator1DXrpBalance, 11_000_000_000n);
        assert.equal(sXrpBalance, 29_000_000_000n);
        assert.equal(dXrpSupply, 11_000_000_000n);
        // 240_000_003
        assert(liquidator1Position.debt >= 110_000_000n
            && liquidator1Position.debt < 110_010_000n);
        assert(liquidator1Position.discounted_collateral >= 240_000_000n
            && liquidator1Position.discounted_collateral < 240_020_000n);
        assert(liquidator1Position.npv > 129_990_000n
            && liquidator1Position.npv < 130_000_000n);
    });

    it("Case 4: Drop the USDC price so Borrower's NPV <= 0", async function () {
        // USDC price is set to 0.9991
        await initPrice(client, "USDC", 9_991_000_000_000_000n);

        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert(borrower1Position.npv < 0n
            && borrower1Position.npv > -50_000n);
    });

    it("Case 5: Liquidator tries to liquidate Borrower's position", async function () {
        await expect(liquidate(client, liquidator1Keys, borrower1Address, "XLM", true))
            .to.eventually.rejected;
    });

    // TODO: requires optimization
    it("Case 6: Liquidator liquidates Borrower's positions partialy", async function () {
        await mintUnderlyingTo(client, "XRP", liquidator1Address, 1_000_000_000n);
        await repay(client, liquidator1Keys, "XRP", I128_MAX);
        // Liquidator1 liquidates Borrower1's positions
        const liquidator1XrpBalanceBefore = await tokenBalanceOf(client, "XRP", liquidator1Address);
        
        await liquidate(client, liquidator1Keys, borrower1Address, "XLM", true);

        const liquidator1XrpBalanceAfter = await tokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1SXrpBalance = await sTokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1USDCBalance = await tokenBalanceOf(client, "USDC", liquidator1Address);
        const liquidator1SUsdcBalance = await sTokenBalanceOf(client, "USDC", liquidator1Address);
        // const liquidator1DXrpBalance = await debtTokenBalanceOf(client, "XRP", liquidator1Address);
        // const liquidator1DUsdcBalance = await debtTokenBalanceOf(client, "USDC", liquidator1Address);

        const borrower1SXrpBalance = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SUsdcBalance = await sTokenBalanceOf(client, "USDC", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);

        const sXrpSupply = await sTokenTotalSupply(client, "XRP");
        const sUsdcSupply = await sTokenTotalSupply(client, "USDC");

        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const dUsdcSupply = await debtTokenTotalSupply(client, "USDC");

        // const liquidator1Position = await accountPosition(client, liquidator1Keys);
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(liquidator1XrpBalanceBefore, liquidator1XrpBalanceAfter);
        assert.equal(liquidator1SXrpBalance, 10_000_000_000n);
        assert.equal(liquidator1USDCBalance, 0n);
        assert.equal(liquidator1SUsdcBalance, 0n);

        assert.equal(borrower1SXrpBalance, 0n);
        assert(borrower1SUsdcBalance >= 10_000_000_000n);
        assert.notEqual(borrower1DXlmBalance, 0n);

        assert.equal(sXrpSupply, 40_000_000_000n);
        assert.equal(sUsdcSupply, 40_000_000_000n);

        assert.notEqual(dXlmSupply, 0n);
        assert.equal(dUsdcSupply, 0n);

        assert.notEqual(borrower1Position.debt, 0n);
        assert(borrower1Position.npv > 0n);
    });

    it("Case 6: Drop the USDC price so Borrower's NPV <= 0", async function () {
        // USDC price is set to 0.45
        await initPrice(client, "USDC", 4_500_000_000_000_000n);

        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert(borrower1Position.npv < 0n);
    });

    it("Case 8: Liquidator liquidates Borrower's positions fully", async function () {
        // Liquidator1 liquidates Borrower1's positions
        const liquidator1XrpBalanceBefore = await tokenBalanceOf(client, "XRP", liquidator1Address);
        const borrower1SUsdcBalanceBefore = await sTokenBalanceOf(client, "USDC", borrower1Address);

        await liquidate(client, liquidator1Keys, borrower1Address, "XLM", true);

        const liquidator1XrpBalanceAfter = await tokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1SXrpBalance = await sTokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1USDCBalance = await tokenBalanceOf(client, "USDC", liquidator1Address);
        const liquidator1SUsdcBalance = await sTokenBalanceOf(client, "USDC", liquidator1Address);
        // const liquidator1DXrpBalance = await debtTokenBalanceOf(client, "XRP", liquidator1Address);
        // const liquidator1DUsdcBalance = await debtTokenBalanceOf(client, "USDC", liquidator1Address);

        const borrower1SXrpBalance = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SUsdcBalanceAfter = await sTokenBalanceOf(client, "USDC", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);

        const sXrpSupply = await sTokenTotalSupply(client, "XRP");
        const sUsdcSupply = await sTokenTotalSupply(client, "USDC");

        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const dUsdcSupply = await debtTokenTotalSupply(client, "USDC");

        // const liquidator1Position = await accountPosition(client, liquidator1Keys);
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(liquidator1XrpBalanceAfter, liquidator1XrpBalanceBefore);
        assert.equal(liquidator1SXrpBalance, 10_000_000_000n);
        assert.equal(liquidator1USDCBalance, 0n);
        assert.notEqual(liquidator1SUsdcBalance, 0n);

        assert.equal(borrower1SXrpBalance, 0n);
        assert(borrower1SUsdcBalanceAfter < borrower1SUsdcBalanceBefore);
        assert.equal(borrower1DXlmBalance, 0n);

        assert.equal(sXrpSupply, 40_000_000_000n);
        assert.equal(sUsdcSupply, 40_000_000_000n);

        assert.equal(dXlmSupply, 0n);
        assert.equal(dUsdcSupply, 0n);

        assert.equal(borrower1Position.debt, 0n);
        assert(borrower1Position.npv >= 0n);
    });
});
