import { SorobanClient, delay } from "../soroban.client";
import {
    accountPosition,
    borrow,
    cleanSlenderEnvKeys,
    debtTokenBalanceOf,
    debtTokenTotalSupply,
    deploy,
    deposit,
    inPoolBalanceOf,
    init,
    initPrice,
    liquidate,
    mintUnderlyingTo,
    sTokenBalanceOf,
    sTokenTotalSupply,
    sTokenUnderlyingBalanceOf,
    tokenBalanceOf
} from "../pool.sut";
import { borrower1Keys, contractsFilename, lender1Keys, liquidator1Keys } from "../soroban.config";
import { assert, use } from "chai";
import chaiAsPromised from 'chai-as-promised';
use(chaiAsPromised);

describe("LendingPool: Liquidation (RWA)", function () {
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
        await mintUnderlyingTo(client, "XRP", borrower1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "RWA", borrower1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "XLM", liquidator1Address, 1_000_000_000n);
    });

    it("Case 1: Liquidator, Lender & Borrower deposit assets", async function () {
        // Lender1 deposits 10.010 XLM
        await deposit(client, lender1Keys, "XLM", 100_000_000n);

        // Borrower1 deposits 10.0 XRP
        await deposit(client, borrower1Keys, "XRP", 10_000_000_000n);
        // Borrower1 deposits 10.0 RWA
        await deposit(client, borrower1Keys, "RWA", 10_000_000_000n);

        // Liquidator1 deposits 20.0 XLM
        await deposit(client, liquidator1Keys, "XLM", 200_000_000n);

        const lender1XlmBalance = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalance = await sTokenBalanceOf(client, "XLM", lender1Address);

        const borrower1XrpBalance = await tokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SXrpBalance = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1RWABalance = await inPoolBalanceOf(client, "RWA", borrower1Address);

        const liquidator1XlmBalance = await tokenBalanceOf(client, "XLM", liquidator1Address);
        const liquidator1SXlmBalance = await sTokenBalanceOf(client, "XLM", liquidator1Address);

        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");

        const sXlmSupply = await sTokenTotalSupply(client, "XLM");
        const sXrpSupply = await sTokenTotalSupply(client, "XRP");

        assert.equal(lender1XlmBalance, 900_000_000n);
        assert.equal(lender1SXlmBalance, 100_000_000n);

        assert.equal(borrower1XrpBalance, 90_000_000_000n);
        assert.equal(borrower1SXrpBalance, 10_000_000_000n);
        assert.equal(borrower1RWABalance, 10_000_000_000n);

        assert.equal(liquidator1XlmBalance, 800_000_000n);
        assert.equal(liquidator1SXlmBalance, 200_000_000n);

        assert.equal(sXlmBalance, 300_000_000n);
        assert.equal(sXrpBalance, 10_000_000_000n);

        assert.equal(sXlmSupply, 300_000_000n);
        assert.equal(sXrpSupply, 10_000_000_000n);
    });

    it("Case 2: Borrower borrows XLM with health ~= initial_health", async function () {
        await delay(20_000);

        // Borrower1 borrows 9.0 XLM
        await borrow(client, borrower1Keys, "XLM", 90_000_000n);

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XlmBalance, 90_000_000n);
        assert.equal(borrower1DXlmBalance, 90_000_001n);
        assert.equal(sXlmBalance, 210000000n);
        assert.equal(dXlmSupply, 90_000_001n);

        assert(borrower1Position.debt > 90_000_000n
            && borrower1Position.debt < 120_000_000n);
        assert.equal(borrower1Position.discounted_collateral, 120_000_000n);
        assert(borrower1Position.npv > 0
            && borrower1Position.npv < 90_000_000n);
    });

    it("Case 4: Drop the XRP price so Borrower's NPV <= 0", async function () {
        // XRP price is set to 0.1
        await initPrice(client, "XRP", 1_000_000_000_000_000n, 0);

        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert(borrower1Position.npv < 0n);
    });

    it("Case 5: Liquidator liquidates Borrower's position", async function () {
        // Liquidator1 liquidates Borrower1's positions
        const liquidator1RWABalanceBefore = await tokenBalanceOf(client, "RWA", liquidator1Address);
        const liquidator1XrpBalanceBefore = await tokenBalanceOf(client, "XRP", liquidator1Address);
        const borrower1SXrpBalanceBefore = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1RWABalanceBefore = await inPoolBalanceOf(client, "RWA", borrower1Address);
        const dXlmSupplyBefore = await debtTokenTotalSupply(client, "XLM");
        const borrower1DXlmBalanceBefore = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1PositionBefore = await accountPosition(client, borrower1Keys);

        console.log("liquidator1Address", liquidator1Address);
        console.log("borrower1Address", borrower1Address);
        await liquidate(client, liquidator1Keys, borrower1Address, false);

        const liquidator1XrpBalanceAfter = await tokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1SXrpBalance = await sTokenBalanceOf(client, "XRP", liquidator1Address);
        const liquidator1RWABalanceAfter = await tokenBalanceOf(client, "RWA", liquidator1Address);

        const borrower1SXrpBalanceAfter = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1RWABalanceAfter = await inPoolBalanceOf(client, "RWA", borrower1Address);
        const borrower1DXlmBalanceAfter = await debtTokenBalanceOf(client, "XLM", borrower1Address);

        const dXlmSupplyAfter = await debtTokenTotalSupply(client, "XLM");

        const borrower1PositionAfter = await accountPosition(client, borrower1Keys);

        assert(liquidator1XrpBalanceAfter > liquidator1XrpBalanceBefore);
        assert.equal(liquidator1SXrpBalance, 0n);
        assert(liquidator1RWABalanceBefore < liquidator1RWABalanceAfter);

        assert(borrower1RWABalanceBefore > borrower1RWABalanceAfter);
        assert(borrower1SXrpBalanceBefore > borrower1SXrpBalanceAfter);
        assert(borrower1DXlmBalanceBefore > borrower1DXlmBalanceAfter);

        assert(dXlmSupplyAfter < dXlmSupplyBefore);

        assert(borrower1PositionAfter.debt < borrower1PositionBefore.debt);
        assert(borrower1PositionAfter.discounted_collateral < borrower1PositionBefore.discounted_collateral);
    });
});

