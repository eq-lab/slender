import { SorobanClient, delay } from "../soroban.client";
import {
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
    sTokenBalanceOf,
    sTokenTotalSupply,
    sTokenUnderlyingBalanceOf,
    tokenBalanceOf
} from "../pool.sut";
import { borrower1Keys, lender1Keys, liquidator1Keys } from "../soroban.config";
import { assert, use } from "chai";
import chaiAsPromised from 'chai-as-promised';
use(chaiAsPromised);

describe("LendingPool: Liquidation (receive underlying assets)", function () {
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
        await mintUnderlyingTo(client, "XRP", borrower1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "USDC", borrower1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "XLM", liquidator1Address, 1_000_000_000n);
    });

    it("Case 1: Liquidator, Lender & Borrower deposit assets", async function () {
        // Lender1 deposits 100_000_000n XLM
        await deposit(client, lender1Keys, "XLM", 100_000_000n);

        // Borrower1 deposits 10_000_000_000 XRP
        await deposit(client, borrower1Keys, "XRP", 10_000_000_000n);
        // Borrower1 deposits 10_000_000_000 USDC
        await deposit(client, borrower1Keys, "USDC", 10_000_000_000n);

        // Liquidator1 deposits 200_000_000n XLM
        await deposit(client, liquidator1Keys, "XLM", 200_000_000n);

        const lender1XlmBalance = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalance = await sTokenBalanceOf(client, "XLM", lender1Address);

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

        assert.equal(lender1XlmBalance, 900_000_000n);
        assert.equal(lender1SXlmBalance, 100_000_000n);

        assert.equal(borrower1XrpBalance, 90_000_000_000n);
        assert.equal(borrower1SXrpBalance, 10_000_000_000n);
        assert.equal(borrower1UsdcBalance, 90_000_000_000n);
        assert.equal(borrower1SUsdcBalance, 10_000_000_000n);

        assert.equal(liquidator1XlmBalance, 800_000_000n);
        assert.equal(liquidator1SXlmBalance, 200_000_000n);

        assert.equal(sXlmBalance, 300_000_000n);
        assert.equal(sXrpBalance, 10_000_000_000n);
        assert.equal(sUsdcBalance, 10_000_000_000n);

        assert.equal(sXlmSupply, 300_000_000n);
        assert.equal(sXrpSupply, 10_000_000_000n);
        assert.equal(sUsdcSupply, 10_000_000_000n);
    });

    it("Case 2: Borrower borrows XLM with npv ~= 0", async function () {
        await delay(20_000);

        // Borrower1 borrows 11_999_000_000n XLM
        await borrow(client, borrower1Keys, "XLM", 119_990_000n);

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XlmBalance, 119_990_000n);
        assert.equal(borrower1DXlmBalance, 119_990_000n);
        assert.equal(sXlmBalance, 180_010_000n);
        assert.equal(dXlmSupply, 119_990_000n);

        assert(borrower1Position.debt > 119_990_000n
            && borrower1Position.debt < 120_000_000n);
        assert.equal(borrower1Position.discounted_collateral, 120_000_000n);
        assert(borrower1Position.npv > 0
            && borrower1Position.npv < 10_000n);
    });

    it("Case 3: Liquidator borrows USDC with npv > 0", async function () {
        // Liquidator1 borrows 1_000_000_000 USDC
        await borrow(client, liquidator1Keys, "USDC", 1_000_000_000n);

        const liquidator1UsdcBalance = await tokenBalanceOf(client, "USDC", liquidator1Address);
        const liquidator1DUsdcBalance = await debtTokenBalanceOf(client, "USDC", liquidator1Address);
        const sUsdcBalance = await sTokenUnderlyingBalanceOf(client, "USDC");
        const dUsdcSupply = await debtTokenTotalSupply(client, "USDC");
        const liquidator1Position = await accountPosition(client, liquidator1Keys);

        assert.equal(liquidator1UsdcBalance, 1_000_000_000n);
        assert.equal(liquidator1DUsdcBalance, 1_000_000_000n);
        assert.equal(sUsdcBalance, 9_000_000_000n);
        assert.equal(dUsdcSupply, 1_000_000_000n);

        assert(liquidator1Position.debt >= 10_000_000n
            && liquidator1Position.debt < 10_010_000n);
        assert(liquidator1Position.discounted_collateral >= 120_000_000n
            && liquidator1Position.discounted_collateral < 120_010_000n);
        assert(liquidator1Position.npv >= 110_000_000n
            && liquidator1Position.npv < 110_010_000n);
    });

    it("Case 4: Drop the XRP price so Borrower's NPV <= 0", async function () {
        // XRP price is set to 999_800_000
        await initPrice(client, "XRP", 9_998_000_000_000_000n);

        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert(borrower1Position.npv < 0n
            && borrower1Position.npv > -10_000n);
    });

    it("Case 5: Liquidator liquidates Borrower's position partialy", async function () {
        // Liquidator1 liquidates Borrower1's positions
        const liquidator1USDCBalanceBefore = await tokenBalanceOf(client, "USDC", liquidator1Address);
        const borrower1SUsdcBalanceBefore = await sTokenBalanceOf(client, "USDC", borrower1Address);
        const dXlmSupplyBefore = await debtTokenTotalSupply(client, "XLM");
        const borrower1DXlmBalanceBefore = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1PositionBefore = await accountPosition(client, borrower1Keys);

        await liquidate(client, liquidator1Keys, borrower1Address, "XLM", false);

        const liquidator1XrpBalance = await tokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1SXrpBalance = await sTokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1USDCBalanceAfter = await tokenBalanceOf(client, "USDC", liquidator1Address);
        const liquidator1SUsdcBalanceAfter = await sTokenBalanceOf(client, "USDC", liquidator1Address);
        const liquidator1DUsdcBalance = await debtTokenBalanceOf(client, "USDC", liquidator1Address);

        const borrower1SXrpBalance = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SUsdcBalanceAfter = await sTokenBalanceOf(client, "USDC", borrower1Address);
        const borrower1DXlmBalanceAfter = await debtTokenBalanceOf(client, "XLM", borrower1Address);

        const dXlmSupplyAfter = await debtTokenTotalSupply(client, "XLM");
        const dUsdcSupply = await debtTokenTotalSupply(client, "USDC");

        const borrower1PositionAfter = await accountPosition(client, borrower1Keys);

        assert.equal(liquidator1XrpBalance, 10_000_000_000n);
        assert.equal(liquidator1SXrpBalance, 0n);
        assert.equal(liquidator1USDCBalanceBefore, liquidator1USDCBalanceAfter);
        assert.equal(liquidator1SUsdcBalanceAfter, 0n);
        assert.equal(liquidator1DUsdcBalance, 1_000_000_000n);

        assert.equal(borrower1SXrpBalance, 0n);
        assert.equal(borrower1SUsdcBalanceBefore, borrower1SUsdcBalanceAfter);
        assert(borrower1DXlmBalanceAfter < borrower1DXlmBalanceBefore);

        assert(dXlmSupplyAfter < dXlmSupplyBefore);
        assert.equal(dUsdcSupply, 1_000_000_000n);

        assert(borrower1PositionAfter.debt < borrower1PositionBefore.debt);
    });

    it("Case 6: Drop the USDC price so Borrower's NPV <= 0", async function () {
        // USDC price is set to 0.45
        await initPrice(client, "USDC", 4_500_000_000_000_000n);

        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert(borrower1Position.npv < 0n);
    });

    it("Case 7: Liquidator liquidates Borrower's position fully", async function () {
        // Liquidator1 liquidates Borrower1's positions
        const liquidator1XrpBalanceBefore = await tokenBalanceOf(client, "XRP", liquidator1Address);
        const borrower1SXrpBalanceBefore = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const liquidator1UsdcBalanceBefore = await tokenBalanceOf(client, "USDC", liquidator1Address);
        const borrower1SUsdcBalanceBefore = await sTokenBalanceOf(client, "USDC", borrower1Address);

        await liquidate(client, liquidator1Keys, borrower1Address, "XLM", false);

        const liquidator1UsdcBalanceAfter = await tokenBalanceOf(client, "USDC", liquidator1Address);
        const liquidator1SUsdcBalance = await sTokenBalanceOf(client, "USDC", liquidator1Address);
        const liquidator1XrpBalanceAfter = await tokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1SXrpBalance = await sTokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1DXrpBalance = await debtTokenBalanceOf(client, "XRP", liquidator1Address);

        const borrower1SUsdcBalanceAfter = await sTokenBalanceOf(client, "USDC", borrower1Address);
        const borrower1SXrpBalanceAfter = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);

        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const dUsdcSupply = await debtTokenTotalSupply(client, "USDC");

        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert(liquidator1UsdcBalanceAfter > liquidator1UsdcBalanceBefore);
        assert.equal(liquidator1SUsdcBalance, 0n);
        assert.equal(liquidator1XrpBalanceBefore, liquidator1XrpBalanceAfter);
        assert.equal(liquidator1SXrpBalance, 0n);
        assert.equal(liquidator1DXrpBalance, 0n);

        assert(borrower1SUsdcBalanceAfter < borrower1SUsdcBalanceBefore);
        assert.equal(borrower1SXrpBalanceBefore, borrower1SXrpBalanceAfter);
        assert.equal(borrower1DXlmBalance, 0n);

        assert.equal(dXlmSupply, 0n);
        assert.equal(dUsdcSupply, 1_000_000_000n);

        assert.equal(borrower1Position.debt, 0n);
    });
});

