import { SorobanClient } from "../soroban.client";
import {
    borrow,
    collatCoeff,
    debtTokenBalanceOf,
    debtTokenTotalSupply,
    deposit,
    init,
    mintUnderlyingTo,
    repay,
    sTokenBalanceOf,
    sTokenTotalSupply,
    sTokenUnderlyingBalanceOf,
    tokenBalanceOf,
    withdraw
} from "../pool.sut";
import {
    borrower1Keys,
    borrower2Keys,
    lender1Keys,
    lender2Keys,
    treasuryKeys
} from "../soroban.config";
import { assert, expect, use } from "chai";
import chaiAsPromised from 'chai-as-promised';
use(chaiAsPromised);

describe("LendingPool", function () {
    let client: SorobanClient;
    let lender1Address: string;
    let borrower1Address: string;
    let lender2Address: string;
    let borrower2Address: string;
    let treasuryAddress: string;

    before(async function () {
        client = new SorobanClient();
        await init(client);

        lender1Address = lender1Keys.publicKey();
        lender2Address = lender2Keys.publicKey();
        borrower1Address = borrower1Keys.publicKey();
        borrower2Address = borrower2Keys.publicKey();
        treasuryAddress = treasuryKeys.publicKey();

        await client.registerAccount(lender1Address);
        await client.registerAccount(lender2Address);
        await client.registerAccount(borrower1Address);
        await client.registerAccount(borrower2Address);

        await mintUnderlyingTo(client, "XLM", lender1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "XRP", lender2Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "USDC", borrower1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "USDC", borrower2Address, 100_000_000_000n);
    });

    it("Case 1: Lenders & borrowers deposit into pool", async function () {
        // Lender1 deposits 10_000_000_000 XLM
        await deposit(client, lender1Keys, "XLM", 10_000_000_000n);

        // Lender2 deposits 10_000_000_000 XRP
        await deposit(client, lender2Keys, "XRP", 10_000_000_000n);

        // Borrower1 deposits 10_000_000_000 USDC
        await deposit(client, borrower1Keys, "USDC", 20_000_000_000n);

        // Borrower2 deposits 10_000_000_000 USDC
        await deposit(client, borrower2Keys, "USDC", 20_000_000_000n);

        const lender1XlmBalanceResult = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalanceResult = await sTokenBalanceOf(client, "XLM", lender1Address);
        const lender2XrpBalanceResult = await tokenBalanceOf(client, "XRP", lender2Address);
        const lender2SXrpBalanceResult = await sTokenBalanceOf(client, "XRP", lender2Address);

        const borrower1UsdcBalanceResult = await tokenBalanceOf(client, "USDC", borrower1Address);
        const borrower1SUsdcBalanceResult = await sTokenBalanceOf(client, "USDC", borrower1Address);
        const borrower2UsdcBalanceResult = await tokenBalanceOf(client, "USDC", borrower2Address);
        const borrower2SUsdcBalanceResult = await sTokenBalanceOf(client, "USDC", borrower2Address);

        const sXlmBalanceResult = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalanceResult = await sTokenUnderlyingBalanceOf(client, "XRP");
        const sUsdcBalanceResult = await sTokenUnderlyingBalanceOf(client, "USDC");

        const sXlmSupplyResult = await sTokenTotalSupply(client, "XLM");
        const sXrpSupplyResult = await sTokenTotalSupply(client, "XRP");
        const sUsdcSupplyResult = await sTokenTotalSupply(client, "USDC");

        assert.equal(lender1XlmBalanceResult, 90_000_000_000n);
        assert.equal(lender1SXlmBalanceResult, 10_000_000_000n);
        assert.equal(lender2XrpBalanceResult, 90_000_000_000n);
        assert.equal(lender2SXrpBalanceResult, 10_000_000_000n);

        assert.equal(borrower1UsdcBalanceResult, 80_000_000_000n);
        assert.equal(borrower1SUsdcBalanceResult, 20_000_000_000n);
        assert.equal(borrower2UsdcBalanceResult, 80_000_000_000n);
        assert.equal(borrower2SUsdcBalanceResult, 20_000_000_000n);

        assert.equal(sXlmBalanceResult, 10_000_000_000n);
        assert.equal(sXrpBalanceResult, 10_000_000_000n);
        assert.equal(sUsdcBalanceResult, 40_000_000_000n);

        assert.equal(sXlmSupplyResult, 10_000_000_000n);
        assert.equal(sXrpSupplyResult, 10_000_000_000n);
        assert.equal(sUsdcSupplyResult, 40_000_000_000n);
    });

    it("Case 2: Borrowers borrow assets from pool with max utilization", async function () {
        // Borrower1 borrows 10_000_000_000 XLM
        await borrow(client, borrower1Keys, "XLM", 9_000_000_000n);

        // Borrower2 borrows 10_000_000_000 XRP
        await borrow(client, borrower2Keys, "XRP", 9_000_000_000n);

        const borrower1XlmBalanceResult = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower2XrpBalanceResult = await tokenBalanceOf(client, "XRP", borrower2Address);

        const borrower1DXlmBalanceResult = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const borrower2DXrpBalanceResult = await debtTokenBalanceOf(client, "XRP", borrower2Address);

        const sXlmBalanceResult = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalanceResult = await sTokenUnderlyingBalanceOf(client, "XRP");

        const dXlmSupplyResult = await debtTokenTotalSupply(client, "XLM");
        const dXrpSupplyResult = await debtTokenTotalSupply(client, "XRP");

        assert.equal(borrower1XlmBalanceResult, 9_000_000_000n);
        assert.equal(borrower2XrpBalanceResult, 9_000_000_000n);

        assert.equal(borrower1DXlmBalanceResult, 9_000_000_000n);
        assert.equal(borrower2DXrpBalanceResult, 9_000_000_000n);

        assert.equal(sXlmBalanceResult, 1_000_000_000n);
        assert.equal(sXrpBalanceResult, 1_000_000_000n);

        assert.equal(dXlmSupplyResult, 9_000_000_000n);
        assert.equal(dXrpSupplyResult, 9_000_000_000n);
    });

    it("Case 3: Borrowers try to borrow more when max utilization exceeded", async function () {
        // Borrower1 borrows 1_000_000_000 XLM
        await expect(borrow(client, borrower1Keys, "XLM", 1_000_000_000n)).to.eventually.rejected;

        // Borrower2 borrows 1_000_000_000 XRP
        await expect(borrow(client, borrower2Keys, "XRP", 1_000_000_000n)).to.eventually.rejected;

        const borrower1XlmBalanceResult = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower2XrpBalanceResult = await tokenBalanceOf(client, "XRP", borrower2Address);

        const sXlmBalanceResult = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalanceResult = await sTokenUnderlyingBalanceOf(client, "XRP");

        assert.equal(borrower1XlmBalanceResult, 9_000_000_000n);
        assert.equal(borrower2XrpBalanceResult, 9_000_000_000n);

        assert.equal(sXlmBalanceResult, 1_000_000_000n);
        assert.equal(sXrpBalanceResult, 1_000_000_000n);
    });

    it("Case 4: Collateral coefficient should be increased as time goes", async function () {
        const xlmCollatCoeff = await collatCoeff(client, "XLM");
        const xrpCollatCoeff = await collatCoeff(client, "XRP");
        const usdcCollatCoeff = await collatCoeff(client, "USDC");

        assert(xlmCollatCoeff > 1_000_000_000n);
        assert(xrpCollatCoeff > 1_000_000_000n);
        assert(usdcCollatCoeff == 1_000_000_000n);
    });

    it("Case 5: Lenders withdraw to make utilization ~ 1", async function () {
        // Lender1 withdraws 1_000_000_000 XLM
        await withdraw(client, lender1Keys, "XLM", 1_000_000_000n);

        // Lender2 withdraws 1_000_000_000 XRP
        await withdraw(client, lender2Keys, "XRP", 1_000_000_000n);

        const lender1XlmBalanceResult = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalanceResult = await sTokenBalanceOf(client, "XLM", lender1Address);
        const lender2XrpBalanceResult = await tokenBalanceOf(client, "XRP", lender2Address);
        const lender2SXrpBalanceResult = await sTokenBalanceOf(client, "XRP", lender2Address);

        const sXlmBalanceResult = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalanceResult = await sTokenUnderlyingBalanceOf(client, "XRP");

        const sXlmSupplyResult = await sTokenTotalSupply(client, "XLM");
        const sXrpSupplyResult = await sTokenTotalSupply(client, "XRP");

        assert.equal(lender1XlmBalanceResult, 91_000_000_000n);
        assert(lender1SXlmBalanceResult < 9_001_000_000n
            && lender1SXlmBalanceResult > 9_000_000_000n);
        assert.equal(lender2XrpBalanceResult, 91_000_000_000n);
        assert(lender2SXrpBalanceResult < 9_001_000_000n
            && lender2SXrpBalanceResult > 9_000_000_000n);

        assert.equal(sXlmBalanceResult, 0n);
        assert.equal(sXrpBalanceResult, 0n);

        assert.equal(sXlmSupplyResult, lender1SXlmBalanceResult);
        assert.equal(sXrpSupplyResult, lender2SXrpBalanceResult);
    });

    it("Case 6: Lenders try to make overwithdraw when utilization ~ 1", async function () {
        // Lender1 withdraws 1_000_000_000 XLM
        await expect(withdraw(client, lender1Keys, "XLM", 1_000_000_000n)).to.eventually.rejected;

        // Lender2 withdraws 1_000_000_000 XRP
        await expect(withdraw(client, lender2Keys, "XRP", 1_000_000_000n)).to.eventually.rejected;

        const lender1XlmBalanceResult = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalanceResult = await sTokenBalanceOf(client, "XLM", lender1Address);
        const lender2XrpBalanceResult = await tokenBalanceOf(client, "XRP", lender2Address);
        const lender2SXrpBalanceResult = await sTokenBalanceOf(client, "XRP", lender2Address);

        const sXlmBalanceResult = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalanceResult = await sTokenUnderlyingBalanceOf(client, "XRP");

        const sXlmSupplyResult = await sTokenTotalSupply(client, "XLM");
        const sXrpSupplyResult = await sTokenTotalSupply(client, "XRP");

        assert.equal(lender1XlmBalanceResult, 91_000_000_000n);
        assert.equal(lender2XrpBalanceResult, 91_000_000_000n);

        assert.equal(sXlmBalanceResult, 0n);
        assert.equal(sXrpBalanceResult, 0n);

        assert.equal(sXlmSupplyResult, lender1SXlmBalanceResult);
        assert.equal(sXrpSupplyResult, lender2SXrpBalanceResult);
    });

    it("Case 7: Borrower1 makes partial repay", async function () {
        // Borrower1 repays 1_000_000_000 XLM
        await repay(client, borrower1Keys, "XLM", 1_000_000_000n);

        const borrower1XlmBalanceResult = await tokenBalanceOf(client, "XLM", borrower1Address);
        const treasuryXlmBalanceResult = await tokenBalanceOf(client, "XLM", treasuryAddress);
        const borrower1DXlmBalanceResult = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalanceResult = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupplyResult = await debtTokenTotalSupply(client, "XLM");

        assert.equal(borrower1XlmBalanceResult, 8_000_000_000n);
        assert(treasuryXlmBalanceResult > 0 && treasuryXlmBalanceResult < 100_000n);
        assert(borrower1DXlmBalanceResult > 8_000_000_000n
            && borrower1DXlmBalanceResult < 8_001_000_000n);
        assert.equal(sXlmBalanceResult + treasuryXlmBalanceResult, 1_000_000_000n);
        assert.equal(dXlmSupplyResult, borrower1DXlmBalanceResult);
    });

    it("Case 8: Borrower1 makes full repay", async function () {
        // Borrower1 repays 9_000_000_000 XLM
        await mintUnderlyingTo(client, "XLM", borrower1Address, 1_000_000_000n);
        await repay(client, borrower1Keys, "XLM", 9_000_000_000n);

        const borrower1XlmBalanceResult = await tokenBalanceOf(client, "XLM", borrower1Address);
        const treasuryXlmBalanceResult = await tokenBalanceOf(client, "XLM", treasuryAddress);
        const borrower1DXlmBalanceResult = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalanceResult = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupplyResult = await debtTokenTotalSupply(client, "XLM");

        assert(borrower1XlmBalanceResult < 1_000_000_000n
            && borrower1XlmBalanceResult > 999_000_000n);
        assert(treasuryXlmBalanceResult > 0 && treasuryXlmBalanceResult < 100_000n);
        assert.equal(borrower1DXlmBalanceResult, 0n);
        assert(sXlmBalanceResult + treasuryXlmBalanceResult > 9_000_000_000n
            && sXlmBalanceResult + treasuryXlmBalanceResult < 9_001_000_000n);
        assert.equal(dXlmSupplyResult, borrower1DXlmBalanceResult);
    });

    it("Case 9: Borrower2 makes full repay", async function () {
        // Borrower2 repays 10_000_000_000 XRP
        await mintUnderlyingTo(client, "XRP", borrower2Address, 1_000_000_000n);
        await repay(client, borrower2Keys, "XRP", 10_000_000_000n);

        const borrower2XrpBalanceResult = await tokenBalanceOf(client, "XRP", borrower2Address);
        const treasuryXrpBalanceResult = await tokenBalanceOf(client, "XRP", treasuryAddress);
        const borrower2DXrpBalanceResult = await debtTokenBalanceOf(client, "XRP", borrower2Address);
        const sXrpBalanceResult = await sTokenUnderlyingBalanceOf(client, "XRP");
        const dXrpSupplyResult = await debtTokenTotalSupply(client, "XRP");

        assert(borrower2XrpBalanceResult < 1_000_000_000n
            && borrower2XrpBalanceResult > 999_000_000n);
        assert(treasuryXrpBalanceResult > 0 && treasuryXrpBalanceResult < 100_000n);
        assert.equal(borrower2DXrpBalanceResult, 0n);
        assert(sXrpBalanceResult + treasuryXrpBalanceResult > 9_000_000_000n
            && sXrpBalanceResult + treasuryXrpBalanceResult < 9_001_000_000n);
        assert.equal(dXrpSupplyResult, borrower2DXrpBalanceResult);
    });

    it("Case 10: Lenders make partial withdraw", async function () {
        // Lender1 withdraws 1_000_000_000 XLM
        await withdraw(client, lender1Keys, "XLM", 1_000_000_000n);

        // Lender2 withdraws 1_000_000_000 XRP
        await withdraw(client, lender2Keys, "XRP", 1_000_000_000n);

        const lender1XlmBalanceResult = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalanceResult = await sTokenBalanceOf(client, "XLM", lender1Address);
        const lender2XrpBalanceResult = await tokenBalanceOf(client, "XRP", lender2Address);
        const lender2SXrpBalanceResult = await sTokenBalanceOf(client, "XRP", lender2Address);

        const sXlmBalanceResult = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalanceResult = await sTokenUnderlyingBalanceOf(client, "XRP");

        const sXlmSupplyResult = await sTokenTotalSupply(client, "XLM");
        const sXrpSupplyResult = await sTokenTotalSupply(client, "XRP");

        assert.equal(lender1XlmBalanceResult, 92_000_000_000n);
        assert(lender1SXlmBalanceResult < 8_001_000_000n
            && lender1SXlmBalanceResult > 8_000_000_000n);
        assert.equal(lender2XrpBalanceResult, 92_000_000_000n);
        assert(lender2SXrpBalanceResult < 8_001_000_000n
            && lender2SXrpBalanceResult > 8_000_000_000n);

        assert(sXlmBalanceResult > 8_000_000_000n
            && sXlmBalanceResult < 8_001_000_000n);
        assert(sXrpBalanceResult > 8_000_000_000n
            && sXrpBalanceResult < 8_001_000_000n);

        assert.equal(sXlmSupplyResult, lender1SXlmBalanceResult);
        assert.equal(sXrpSupplyResult, lender2SXrpBalanceResult);
    });

    it("Case 11: Lenders make full withdraw", async function () {
        // Lender1 withdraws 10_000_000_000 XLM
        await withdraw(client, lender1Keys, "XLM", 10_000_000_000n);

        // Lender2 withdraws 10_000_000_000 XRP
        await withdraw(client, lender2Keys, "XRP", 10_000_000_000n);

        const lender1XlmBalanceResult = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalanceResult = await sTokenBalanceOf(client, "XLM", lender1Address);
        const lender2XrpBalanceResult = await tokenBalanceOf(client, "XRP", lender2Address);
        const lender2SXrpBalanceResult = await sTokenBalanceOf(client, "XRP", lender2Address);

        const sXlmBalanceResult = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalanceResult = await sTokenUnderlyingBalanceOf(client, "XRP");

        const sXlmSupplyResult = await sTokenTotalSupply(client, "XLM");
        const sXrpSupplyResult = await sTokenTotalSupply(client, "XRP");

        assert(lender1XlmBalanceResult > 100_000_000_000n
            && lender1XlmBalanceResult < 100_001_000_000n);
        assert.equal(lender1SXlmBalanceResult, 0n);
        assert(lender2XrpBalanceResult > 100_000_000_000n
            && lender2XrpBalanceResult < 100_001_000_000n);
        assert.equal(lender2SXrpBalanceResult, 0n);

        assert(sXlmBalanceResult < 1_000n);
        assert(sXrpBalanceResult < 1_000n);

        assert.equal(sXlmSupplyResult, lender1SXlmBalanceResult);
        assert.equal(sXrpSupplyResult, lender2SXrpBalanceResult);
    });
});
