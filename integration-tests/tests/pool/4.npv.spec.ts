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
    mintUnderlyingTo,
    sTokenBalanceOf,
    sTokenTotalSupply,
    sTokenUnderlyingBalanceOf,
    setPrice,
    tokenBalanceOf,
    withdraw
} from "../pool.sut";
import { borrower1Keys, lender1Keys } from "../soroban.config";
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

        await client.registerAccount(lender1Address);
        await client.registerAccount(borrower1Address);

        await mintUnderlyingTo(client, "XLM", lender1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "XRP", borrower1Address, 100_000_000_000n);
    });

    it("Case 1: Lender & Borrower make deposits", async function () {
        // Lender1 deposits 50_000_000_000 XLM
        await deposit(client, lender1Keys, "XLM", 50_000_000_000n);

        // Borrower1 deposits 10_000_000_000 XRP
        await deposit(client, borrower1Keys, "XRP", 20_000_000_000n);

        const lender1XlmBalance = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalance = await sTokenBalanceOf(client, "XLM", lender1Address);

        const borrower1XrpBalance = await tokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SXrpBalance = await sTokenBalanceOf(client, "XRP", borrower1Address);

        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");

        const sXlmSupply = await sTokenTotalSupply(client, "XLM");
        const sXrpSupply = await sTokenTotalSupply(client, "XRP");

        assert.equal(lender1XlmBalance, 50_000_000_000n);
        assert.equal(lender1SXlmBalance, 50_000_000_000n);

        assert.equal(borrower1XrpBalance, 80_000_000_000n);
        assert.equal(borrower1SXrpBalance, 20_000_000_000n);

        assert.equal(sXlmBalance, 50_000_000_000n);
        assert.equal(sXrpBalance, 20_000_000_000n);

        assert.equal(sXlmSupply, 50_000_000_000n);
        assert.equal(sXrpSupply, 20_000_000_000n);
    });

    it("Case 2: Borrower borrows with NPV > 0", async function () {
        await delay(20_000);

        // Borrower1 borrows 5_000_000_000n XLM
        await borrow(client, borrower1Keys, "XLM", 5_000_000_000n);

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XlmBalance, 5_000_000_000n);
        assert.equal(borrower1DXlmBalance, 5_000_000_000n);
        assert.equal(sXlmBalance, 45_000_000_000n);
        assert.equal(dXlmSupply, 5_000_000_000n);

        assert(borrower1Position.debt > 5_000_000_000n
            && borrower1Position.debt < 5_001_000_000n);
        assert.equal(borrower1Position.discounted_collateral, 12_000_000_000n);
        assert(borrower1Position.npv > 6_999_000_000n
            && borrower1Position.npv < 7_000_000_000n);
    });

    it("Case 3: Borrower borrows more with NPV > 0", async function () {
        // Borrower1 borrows 5_000_000_000n XLM
        await borrow(client, borrower1Keys, "XLM", 5_000_000_000n);

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XlmBalance, 10_000_000_000n);
        assert(borrower1DXlmBalance > 9_999_000_000n
            && borrower1DXlmBalance < 10_000_000_000n);
        assert.equal(sXlmBalance, 40_000_000_000n);
        assert.equal(dXlmSupply, borrower1DXlmBalance);

        assert(borrower1Position.debt > 10_000_000_000n
            && borrower1Position.debt < 10_001_000_000n);
        assert.equal(borrower1Position.discounted_collateral, 12_000_000_000n);
        assert(borrower1Position.npv > 1_999_000_000n
            && borrower1Position.npv < 2_000_000_000n);
    });

    it("Case 4: Borrower withdraws with npv > 0", async function () {
        // Borrower1 withdraws 3_000_000_000n XRP
        await withdraw(client, borrower1Keys, "XRP", 3_000_000_000n);

        const borrower1XrpBalance = await tokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SXrpBalance = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");
        const sXrpSupply = await sTokenTotalSupply(client, "XRP");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XrpBalance, 83_000_000_000n);
        assert.equal(borrower1SXrpBalance, 17_000_000_000n);
        assert.equal(sXrpBalance, 17_000_000_000n);
        assert.equal(sXrpSupply, borrower1SXrpBalance);

        assert(borrower1Position.debt > 10_000_000_000n
            && borrower1Position.debt < 10_001_000_000n);
        assert.equal(borrower1Position.discounted_collateral, 10_200_000_000n);
        assert(borrower1Position.npv > 199_000_000n
            && borrower1Position.npv < 200_000_000n);
    });

    it("Case 5: Drop the XRP price so Borrower's NPV <= 0", async function () {
        // XRP price is set to 900_000_000
        await setPrice(client, "XRP", 900_000_000n);

        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert(borrower1Position.npv < 0n
            && borrower1Position.npv > -1_000_000_000n);
    });

    it("Case 6: Borrower tries to borrow", async function () {
        // Borrower1 borrows 1_000n XLM
        await expect(borrow(client, borrower1Keys, "XLM", 1_000n)).to.eventually.rejected;

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XlmBalance, 10_000_000_000n);
        assert(borrower1DXlmBalance > 9_999_000_000n
            && borrower1DXlmBalance < 10_000_000_000n);
        assert.equal(sXlmBalance, 40_000_000_000n);
        assert.equal(dXlmSupply, borrower1DXlmBalance);

        assert(borrower1Position.debt > 10_000_000_000n
            && borrower1Position.debt < 10_001_000_000n);
        assert.equal(borrower1Position.discounted_collateral, 9_180_000_000n);
        assert(borrower1Position.npv < 0n
            && borrower1Position.npv > -1_000_000_000n);
    });

    it("Case 7: Borrower tries to withdraw", async function () {
        // Borrower1 withdraws 1_000n XRP
        await expect(withdraw(client, borrower1Keys, "XRP", 1_000n)).to.eventually.rejected;

        const borrower1XrpBalance = await tokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SXrpBalance = await sTokenBalanceOf(client, "XRP", borrower1Address);
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");
        const sXrpSupply = await sTokenTotalSupply(client, "XRP");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XrpBalance, 83_000_000_000n);
        assert.equal(borrower1SXrpBalance, 17_000_000_000n);
        assert.equal(sXrpBalance, 17_000_000_000n);
        assert.equal(sXrpSupply, borrower1SXrpBalance);

        assert(borrower1Position.debt > 10_000_000_000n
            && borrower1Position.debt < 10_001_000_000n);
        assert.equal(borrower1Position.discounted_collateral, 9_180_000_000n);
        assert(borrower1Position.npv < 0n
            && borrower1Position.npv > -1_000_000_000n);
    });

    it("Case 8: Borrower deposits more to achieve good NPV", async function () {
        // Borrower1 deposits 3_000_000_000 XRP
        await deposit(client, borrower1Keys, "XRP", 3_500_000_000n);

        const borrower1XrpBalance = await tokenBalanceOf(client, "XRP", borrower1Address);
        const borrower1SXrpBalance = await sTokenBalanceOf(client, "XRP", borrower1Address);

        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");

        const sXrpSupply = await sTokenTotalSupply(client, "XRP");

        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XrpBalance, 79_500_000_000n);
        assert.equal(borrower1SXrpBalance, 20_500_000_000n);

        assert.equal(sXrpBalance, 20_500_000_000n);

        assert.equal(sXrpSupply, 20_500_000_000n);

        assert(borrower1Position.debt > 10_000_000_000n
            && borrower1Position.debt < 10_001_000_000n);
        assert.equal(borrower1Position.discounted_collateral, 11_070_000_000n);
        assert(borrower1Position.npv < 1_100_000_000n
            && borrower1Position.npv > 1_000_000_000n);
    });

    it("Case 9: Borrower borrows more with NPV > 0", async function () {
        // Borrower1 borrows 500_000_000n XLM
        await borrow(client, borrower1Keys, "XLM", 500_000_000n);

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const borrower1Position = await accountPosition(client, borrower1Keys);

        assert.equal(borrower1XlmBalance, 10_500_000_000n);
        assert(borrower1DXlmBalance > 10_000_000_000n
            && borrower1DXlmBalance < 10_500_000_000n);
        assert.equal(sXlmBalance, 39_500_000_000n);
        assert.equal(dXlmSupply, borrower1DXlmBalance);

        assert(borrower1Position.debt > 10_500_000_000n
            && borrower1Position.debt < 10_501_000_000n);
        assert.equal(borrower1Position.discounted_collateral, 11_070_000_000n);
        assert(borrower1Position.npv > 500_000_000n
            && borrower1Position.npv < 1_000_000_000n);
    });
});
