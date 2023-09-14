import { SorobanClient } from "../soroban.client";
import {
    accountPosition,
    borrow,
    cleanSlenderEnvKeys,
    debtTokenBalanceOf,
    debtTokenTotalSupply,
    deploy,
    deposit,
    init,
    liquidate,
    mintUnderlyingTo,
    sTokenBalanceOf,
    sTokenTotalSupply,
    sTokenUnderlyingBalanceOf,
    setPrice,
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

        await client.registerAccount(lender1Address);
        await client.registerAccount(borrower1Address);
        await client.registerAccount(liquidator1Address);

        await mintUnderlyingTo(client, "XLM", lender1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "XRP", borrower1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "USDC", borrower1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "XLM", liquidator1Address, 100_000_000_000n);
    });

    it("Case 1: Liquidator, Lender & Borrower deposit assets", async function () {
        // Lender1 deposits 10_000_000_000 XLM
        await deposit(client, lender1Keys, "XLM", 10_000_000_000n);

        // Borrower1 deposits 10_000_000_000 XRP
        await deposit(client, borrower1Keys, "XRP", 10_000_000_000n);
        // Borrower1 deposits 10_000_000_000 USDC
        await deposit(client, borrower1Keys, "USDC", 10_000_000_000n);

        // Liquidator1 deposits 10_000_000_000 XLM
        await deposit(client, liquidator1Keys, "XLM", 20_000_000_000n);

        const lender1XlmBalance = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalance = await sTokenBalanceOf(client, "XLM", lender1Address);

        const borrower1XrpBalance = await tokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SXrpBalance = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1UsdcBalance = await tokenBalanceOf(client, "USDC", borrower1Address);
        const borrower1SUsdcBalance = await sTokenBalanceOf(client, "USDC", borrower1Address);

        const liquidator1UsdcBalance = await tokenBalanceOf(client, "XLM", liquidator1Address);
        const liquidator1SUsdcBalance = await sTokenBalanceOf(client, "XLM", liquidator1Address);

        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");
        const sUsdcBalance = await sTokenUnderlyingBalanceOf(client, "USDC");

        const sXlmSupply = await sTokenTotalSupply(client, "XLM");
        const sXrpSupply = await sTokenTotalSupply(client, "XRP");
        const sUsdcSupply = await sTokenTotalSupply(client, "USDC");

        assert.equal(lender1XlmBalance, 90_000_000_000n);
        assert.equal(lender1SXlmBalance, 10_000_000_000n);

        assert.equal(borrower1XrpBalance, 90_000_000_000n);
        assert.equal(borrower1SXrpBalance, 10_000_000_000n);
        assert.equal(borrower1UsdcBalance, 90_000_000_000n);
        assert.equal(borrower1SUsdcBalance, 10_000_000_000n);

        assert.equal(liquidator1UsdcBalance, 80_000_000_000n);
        assert.equal(liquidator1SUsdcBalance, 20_000_000_000n);

        assert.equal(sXlmBalance, 30_000_000_000n);
        assert.equal(sXrpBalance, 10_000_000_000n);
        assert.equal(sUsdcBalance, 10_000_000_000n);

        assert.equal(sXlmSupply, 30_000_000_000n);
        assert.equal(sXrpSupply, 10_000_000_000n);
        assert.equal(sUsdcSupply, 10_000_000_000n);
    });

    it("Case 2: Borrower borrows XLM with npv ~= 0", async function () {
        // Borrower1 borrows 11_999_000_000n XLM
        await borrow(client, borrower1Keys, "XLM", 11_999_000_000n);

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XlmBalance, 11_999_000_000n);
        assert.equal(borrower1DXlmBalance, 11_999_000_000n);
        assert.equal(sXlmBalance, 18_001_000_000n);
        assert.equal(dXlmSupply, 11_999_000_000n);

        assert(borrower1Position.debt > 11_999_000_000n
            && borrower1Position.debt < 12_000_000_000n);
        assert.equal(borrower1Position.discounted_collateral, 12_000_000_000n);
        assert(borrower1Position.npv > 0
            && borrower1Position.npv < 1_000_000n);
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

        assert(liquidator1Position.debt > 1_000_000_000n
            && liquidator1Position.debt < 1_001_000_000n);
        assert(liquidator1Position.discounted_collateral > 12_000_000_000n
            && liquidator1Position.discounted_collateral < 12_001_000_000n);
        assert(liquidator1Position.npv > 11_000_000_000n
            && liquidator1Position.npv < 11_001_000_000n);
    });

    it("Case 4: Drop the XRP price so Borrower's NPV <= 0", async function () {
        // XLM price is set to 999_800_000
        await setPrice(client, "XRP", 999_800_000n);

        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert(borrower1Position.npv < 0n
            && borrower1Position.npv > -1_000_000n);
    });

    it("Case 5: Liquidator liquidates Borrower's positions", async function () {
        // Liquidator1 liquidates Borrower1's positions
        await liquidate(client, liquidator1Keys, borrower1Address, false);

        const liquidator1XrpBalance = await tokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1SXrpBalance = await sTokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1USDCBalance = await tokenBalanceOf(client, "USDC", liquidator1Address);
        const liquidator1SUsdcBalance = await sTokenBalanceOf(client, "USDC", liquidator1Address);
        const liquidator1DUsdcBalance = await debtTokenBalanceOf(client, "USDC", liquidator1Address);

        const borrower1SXrpBalance = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SUsdcBalance = await sTokenBalanceOf(client, "USDC", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);

        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const dUsdcSupply = await debtTokenTotalSupply(client, "USDC");

        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(liquidator1XrpBalance, 10_000_000_000n);
        assert.equal(liquidator1SXrpBalance, 0n);
        assert(liquidator1USDCBalance > 4_000_000_000n
            && liquidator1USDCBalance < 5_000_000_000n);
        assert.equal(liquidator1SUsdcBalance, 0n);
        assert.equal(liquidator1DUsdcBalance, 1_000_000_000n);

        assert.equal(borrower1SXrpBalance, 0n);
        assert(borrower1SUsdcBalance > 6_000_000_000n
            && borrower1SUsdcBalance < 7_000_000_000n);
        assert.equal(borrower1DXlmBalance, 0n);

        assert.equal(dXlmSupply, 0n);
        assert.equal(dUsdcSupply, 1_000_000_000n);

        assert.equal(borrower1Position.debt, 0n);
    });
});
