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
    healthFactor,
    init,
    initPrice,
    liquidate,
    mintUnderlyingTo,
    repay,
    sTokenBalanceOf,
    sTokenTotalSupply,
    sTokenUnderlyingBalanceOf,
    tokenBalanceOf,
    withdraw
} from "../pool.sut";
import { borrower1Keys, contractsFilename, lender1Keys, liquidator1Keys } from "../soroban.config";
import { assert, expect, use } from "chai";
import chaiAsPromised from 'chai-as-promised';
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
        
        // uncomment to resume test with existing contracts
        // require("dotenv").config({ path: contractsFilename });
        // return;

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
        // Lender1 deposits 10 XLM
        await deposit(client, lender1Keys, "XLM", 300_000_000n);
        // Lender1 deposits 30 XRP
        await deposit(client, lender1Keys, "XRP", 30_000_000_000n);
        // Lender1 deposits 30 USDC
        await deposit(client, lender1Keys, "USDC", 30_000_000_000n);

        // Borrower1 deposits 10 XRP
        await deposit(client, borrower1Keys, "XRP", 10_000_000_000n);
        // Borrower1 deposits 10 USDC
        await deposit(client, borrower1Keys, "USDC", 10_000_000_000n);

        // Liquidator1 deposits 40 XLM
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

    it("Case 2: Borrower borrows XLM with health >= 0.25", async function () {
        await delay(20_000);

        // Borrower1 borrows 8.9 XLM
        await borrow(client, borrower1Keys, "XLM", 89_000_000n);

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XlmBalance, 89_000_000n);
        assert.equal(borrower1DXlmBalance, 89_000_001n);
        assert.equal(sXlmBalance, 611_000_000n);
        assert.equal(dXlmSupply, 89_000_001n);

        assert(borrower1Position.debt > 89_000_000n
            && borrower1Position.debt < 90_000_000n);
        assert.equal(borrower1Position.discounted_collateral, 120_000_000n);
        assert(borrower1Position.npv > 3_0_000_000n
            && borrower1Position.npv < 3_1_000_000n);
        assert(healthFactor(borrower1Position) >= 0.25);
    });

    it("Case 3: Liquidator borrows XRP", async function () {
        // Liquidator1 borrows 11 XRP
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
        // USDC price is set to 0.48
        await initPrice(client, "USDC", 4_800_000_000_000_000n, 0);

        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert(borrower1Position.npv < 0n
            && borrower1Position.npv > -210_000);
    });

    it("Case 5: Liquidator tries to liquidate Borrower's position", async function () {
        await expect(liquidate(client, liquidator1Keys, borrower1Address, true))
            .to.eventually.rejected;
    });

    // TODO: requires optimization
    it("Case 6: Liquidator liquidates Borrower's positions partialy", async function () {
        await mintUnderlyingTo(client, "XRP", liquidator1Address, 1_000_000_000n);
        await repay(client, liquidator1Keys, "XRP", I128_MAX);
        // Liquidator1 liquidates Borrower1's positions
        const liquidator1XrpBalanceBefore = await tokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1SXrpBalanceBefore = await sTokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1SUsdcBalanceBefore = await sTokenBalanceOf(client, "USDC", liquidator1Address);
        const borrower1SXrpBalanceBefore = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SUsdcBalanceBefore = await sTokenBalanceOf(client, "USDC", borrower1Address);
        
        await liquidate(client, liquidator1Keys, borrower1Address, true);

        const liquidator1XrpBalanceAfter = await tokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1SXrpBalanceAfter = await sTokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1USDCBalance = await tokenBalanceOf(client, "USDC", liquidator1Address);
        const liquidator1SUsdcBalanceAfter = await sTokenBalanceOf(client, "USDC", liquidator1Address);

        const borrower1SXrpBalanceAfter = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SUsdcBalanceAfter = await sTokenBalanceOf(client, "USDC", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);

        const sXrpSupply = await sTokenTotalSupply(client, "XRP");
        const sUsdcSupply = await sTokenTotalSupply(client, "USDC");

        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const dUsdcSupply = await debtTokenTotalSupply(client, "USDC");

        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(liquidator1XrpBalanceBefore, liquidator1XrpBalanceAfter);
        assert(liquidator1SXrpBalanceBefore < liquidator1SXrpBalanceAfter);
        assert.equal(liquidator1USDCBalance, 0n);
        assert(liquidator1SUsdcBalanceBefore <= liquidator1SUsdcBalanceAfter);

        assert(borrower1SXrpBalanceBefore > borrower1SXrpBalanceAfter);
        assert(borrower1SUsdcBalanceBefore >= borrower1SUsdcBalanceAfter);
        assert.notEqual(borrower1DXlmBalance, 0n);

        assert.equal(sXrpSupply, 40_000_000_000n);
        assert.equal(sUsdcSupply, 40_000_000_000n);

        assert.notEqual(dXlmSupply, 0n);
        assert.equal(dUsdcSupply, 0n);

        assert.notEqual(borrower1Position.debt, 0n);
        assert(borrower1Position.npv > 0n);
    });

    it("Case 7: Borrower withdraw XRP partialy to NPV ~= 0", async function () {
        const borrower1SXrpBalanceBefore = await sTokenBalanceOf(client, "XRP", borrower1Address);
        // target balance ~3.3 XRP
        const targetXRPBalance = 3_300_000_000n;
        const toWithdraw = borrower1SXrpBalanceBefore - targetXRPBalance;

        await withdraw(client, borrower1Keys, "XRP", toWithdraw < 0n ? 0n : toWithdraw);
    });

    it("Case 8: Drop the XRP price so Borrower's NPV <= 0", async function () {
        // XRP price is set to 0.4799999
        await initPrice(client, "XRP", 4_799_999_000_000_000n, 0);

        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert(borrower1Position.npv < 0n);
    });

    it("Case 9: Liquidator liquidates Borrower's position partialy 2", async function () {
        // Liquidator1 liquidates Borrower1's positions
        const liquidator1XrpBalanceBefore = await tokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1SXrpBalanceBefore = await sTokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1SUsdcBalanceBefore = await sTokenBalanceOf(client, "USDC", liquidator1Address);
        const borrower1SXrpBalanceBefore = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SUsdcBalanceBefore = await sTokenBalanceOf(client, "USDC", borrower1Address);
        const borrower1DXlmBalanceBefore = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        console.log("borrower1SUsdcBalanceBefore", borrower1SUsdcBalanceBefore);

        const sXrpSupplyBefore = await sTokenTotalSupply(client, "XRP");
        const sUsdcSupplyBefore = await sTokenTotalSupply(client, "USDC");
        const borrower1PositionBefore = await accountPosition(client, borrower1Keys);

        await liquidate(client, liquidator1Keys, borrower1Address, true);

        const liquidator1XrpBalanceAfter = await tokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1SXrpBalanceAfter = await sTokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1USDCBalanceAfter = await tokenBalanceOf(client, "USDC", liquidator1Address);
        const liquidator1SUsdcBalanceAfter = await sTokenBalanceOf(client, "USDC", liquidator1Address);

        const borrower1SXrpBalanceAfter = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SUsdcBalanceAfter = await sTokenBalanceOf(client, "USDC", borrower1Address);
        const borrower1DXlmBalanceAfter = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        console.log("borrower1SUsdcBalanceAfter", borrower1SUsdcBalanceAfter);

        const sXrpSupplyAfter = await sTokenTotalSupply(client, "XRP");
        const sUsdcSupplyAfter = await sTokenTotalSupply(client, "USDC");

        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");

        const borrower1PositionAfter = await accountPosition(client, borrower1Keys);

        assert(borrower1SXrpBalanceBefore > borrower1SXrpBalanceAfter);
        assert(borrower1DXlmBalanceBefore > borrower1DXlmBalanceAfter);
        assert(borrower1SUsdcBalanceBefore > borrower1SUsdcBalanceAfter, `borrower1SUsdcBalanceBefore ${borrower1SUsdcBalanceBefore} borrower1SUsdcBalanceAfter ${borrower1SUsdcBalanceAfter}`);

        assert.equal(liquidator1XrpBalanceAfter, liquidator1XrpBalanceBefore);
        assert(liquidator1SXrpBalanceBefore < liquidator1SXrpBalanceAfter);
        assert.equal(liquidator1USDCBalanceAfter, 0n);
        assert(liquidator1SUsdcBalanceBefore < liquidator1SUsdcBalanceAfter, `liquidator1SUsdcBalanceBefore ${liquidator1SUsdcBalanceBefore} liquidator1SUsdcBalanceAfter ${liquidator1SUsdcBalanceAfter}`);


        assert.equal(sXrpSupplyBefore, sXrpSupplyAfter);
        assert.equal(sUsdcSupplyBefore, sUsdcSupplyAfter);

        assert.equal(dXlmSupply, borrower1DXlmBalanceAfter);

        assert(borrower1PositionBefore.npv <= borrower1PositionAfter.npv);
    });

    it("Case 10: Liquidator cannot liquidate borrower without collateral", async function () {
        const borrower1Position = await accountPosition(client, borrower1Keys);
        if (borrower1Position.discounted_collateral === 0n) {
            await expect(liquidate(client, liquidator1Keys, borrower1Address, true))
                .to.eventually.rejected;
        }
    });
});
