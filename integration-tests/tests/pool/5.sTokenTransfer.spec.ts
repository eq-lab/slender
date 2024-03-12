import { SorobanClient, delay } from "../soroban.client";
import {
    accountPosition,
    borrow,
    cleanSlenderEnvKeys,
    deploy,
    deposit,
    init,
    mintUnderlyingTo,
    sTokenBalanceOf,
    sTokenTotalSupply,
    sTokenUnderlyingBalanceOf,
    tokenBalanceOf,
    transferStoken,
} from "../pool.sut";
import { borrower1Keys, borrower2Keys, lender1Keys, lender2Keys, lender3Keys } from "../soroban.config";
import { assert, expect, use } from "chai";
import chaiAsPromised from 'chai-as-promised';
use(chaiAsPromised);

describe("sToken transfer", function () {
    let client: SorobanClient;
    let lender1Address: string;
    let borrower1Address: string;
    let lender2Address: string;
    let borrower2Address: string;
    let lender3Address: string;

    before(async function () {
        client = new SorobanClient();

        await cleanSlenderEnvKeys();
        await deploy();
        await init(client);

        lender1Address = lender1Keys.publicKey();
        borrower1Address = borrower1Keys.publicKey();
        lender2Address = lender2Keys.publicKey();
        borrower2Address = borrower2Keys.publicKey();
        lender3Address = lender3Keys.publicKey();

        // uncomment to resume test with existing contracts
        // require("dotenv").config({ path: contractsFilename });
        // return;

        await Promise.all([
            client.registerAccount(lender1Address),
            client.registerAccount(borrower1Address),
            client.registerAccount(lender2Address),
            client.registerAccount(borrower2Address),
            client.registerAccount(lender3Address),
        ]);

        await mintUnderlyingTo(client, "XLM", lender1Address, 1_000_000_000n);
        await mintUnderlyingTo(client, "XLM", lender2Address, 1_000_000_000n);
        await mintUnderlyingTo(client, "XRP", lender3Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "USDC", borrower1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "USDC", borrower2Address, 100_000_000_000n);
    });

    it("Case 1: Lenders & Borrowers make deposits and borrowings", async function () {
        await deposit(client, lender1Keys, "XLM", 1_000_000_000n);
        await deposit(client, lender2Keys, "XLM", 1_000_000_000n);
        await deposit(client, lender3Keys, "XRP", 100_000_000_000n);
        await deposit(client, borrower1Keys, "USDC", 100_000_000_000n);
        await deposit(client, borrower2Keys, "USDC", 100_000_000_000n);

        await delay(20_000);

        await borrow(client, borrower1Keys, "XLM", 100_000_000n);
        await borrow(client, borrower2Keys, "XRP", 10_000_000_000n);

        const lender1XlmBalance = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalance = await sTokenBalanceOf(client, "XLM", lender1Address);

        const lender2XlmBalance = await tokenBalanceOf(client, "XLM", lender2Address);
        const lender2SXlmBalance = await sTokenBalanceOf(client, "XLM", lender2Address);

        const lender3XrpBalance = await tokenBalanceOf(client, "XRP", lender3Address);
        const lender3SXrpBalance = await sTokenBalanceOf(client, "XRP", lender3Address);

        const borrower1UsdcBalance = await tokenBalanceOf(client, "USDC", borrower1Address);
        const borrower1SUsdcBalance = await sTokenBalanceOf(client, "USDC", borrower1Address);

        const borrower2UsdcBalance = await tokenBalanceOf(client, "USDC", borrower2Address);
        const borrower2SUsdcBalance = await sTokenBalanceOf(client, "USDC", borrower2Address);

        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");
        const sUsdcBalance = await sTokenUnderlyingBalanceOf(client, "USDC");

        const sXlmSupply = await sTokenTotalSupply(client, "XLM");
        const sXrpSupply = await sTokenTotalSupply(client, "XRP");
        const sUsdcSupply = await sTokenTotalSupply(client, "USDC");

        assert.equal(lender1XlmBalance, 0n);
        assert.equal(lender1SXlmBalance, 1_000_000_000n);

        assert.equal(lender2XlmBalance, 0n);
        assert.equal(lender2SXlmBalance, 1_000_000_000n);

        assert.equal(lender3XrpBalance, 0n);
        assert.equal(lender3SXrpBalance, 100_000_000_000n);

        assert.equal(borrower1UsdcBalance, 0n);
        assert.equal(borrower1SUsdcBalance, 100_000_000_000n);

        assert.equal(borrower2UsdcBalance, 0n);
        assert.equal(borrower2SUsdcBalance, 100_000_000_000n);

        assert.equal(sXlmBalance, 1_900_000_000n);
        assert.equal(sXrpBalance, 90_000_000_000n);
        assert.equal(sUsdcBalance, 200_000_000_000n);

        assert.equal(sXlmSupply, 2_000_000_000n);
        assert.equal(sXrpSupply, 100_000_000_000n);
        assert.equal(sUsdcSupply, 200_000_000_000n);
    });

    it("Case 2: Lender1 transfers sXlm to Lender3 who doesn't have them yet", async function () {
        const sXlmSupplyBefore = await sTokenTotalSupply(client, "XLM");
        const sXlmBalanceBefore = await sTokenUnderlyingBalanceOf(client, "XLM");

        await transferStoken(client, "XLM", lender1Keys, lender3Address, 100_000_000n);

        const sXlmSupplyAfter = await sTokenTotalSupply(client, "XLM");
        const sXlmBalanceAfter = await sTokenUnderlyingBalanceOf(client, "XLM");
        const lender1SXlmBalance = await sTokenBalanceOf(client, "XLM", lender1Address);
        const lender3SXlmBalance = await sTokenBalanceOf(client, "XLM", lender3Address);

        assert.equal(lender1SXlmBalance, 900_000_000n);
        assert.equal(lender3SXlmBalance, 100_000_000n);
        assert.equal(sXlmSupplyBefore, sXlmSupplyAfter);
        assert.equal(sXlmBalanceBefore, sXlmBalanceAfter);
    });

    it("Case 3: Lender1 transfers sXlm to Borrower2 who doesn't have debtXlm", async function () {
        const sXlmSupplyBefore = await sTokenTotalSupply(client, "XLM");
        const sXlmBalanceBefore = await sTokenUnderlyingBalanceOf(client, "XLM");
        const borrower2PositionBefore = await accountPosition(client, borrower2Keys);

        await transferStoken(client, "XLM", lender1Keys, borrower2Address, 100_000_000n);

        const sXlmSupplyAfter = await sTokenTotalSupply(client, "XLM");
        const sXlmBalanceAfter = await sTokenUnderlyingBalanceOf(client, "XLM");
        const lender1SXlmBalance = await sTokenBalanceOf(client, "XLM", lender1Address);
        const borrower2SXlmBalance = await sTokenBalanceOf(client, "XLM", borrower2Address);
        const borrower2PositionAfter = await accountPosition(client, borrower2Keys);

        assert.equal(lender1SXlmBalance, 800_000_000n);
        assert.equal(borrower2SXlmBalance, 100_000_000n);
        assert.equal(sXlmSupplyBefore, sXlmSupplyAfter);
        assert.equal(sXlmBalanceBefore, sXlmBalanceAfter);
        assert(borrower2PositionBefore.npv < borrower2PositionAfter.npv);
    });

    it("Case 4: Lender1 transfers sXlm to Borrower1 who has debtXlm", async function () {
        const sXlmSupplyBefore = await sTokenTotalSupply(client, "XLM");
        const sXlmBalanceBefore = await sTokenUnderlyingBalanceOf(client, "XLM");

        await expect(transferStoken(client, "XLM", lender1Keys, borrower1Address, 100_000_000n)).to.eventually.rejected;

        const sXlmSupplyAfter = await sTokenTotalSupply(client, "XLM");
        const sXlmBalanceAfter = await sTokenUnderlyingBalanceOf(client, "XLM");
        const lender1SXlmBalance = await sTokenBalanceOf(client, "XLM", lender1Address);
        const borrower1SXlmBalance = await sTokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1PositionAfter = await accountPosition(client, borrower1Keys);

        assert.equal(lender1SXlmBalance, 800_000_000n);
        assert.equal(borrower1SXlmBalance, 0n);
        assert.equal(sXlmSupplyBefore, sXlmSupplyAfter);
        assert.equal(sXlmBalanceBefore, sXlmBalanceAfter);
        assert(borrower1PositionAfter.npv > 0);
    });

    it("Case 5: Borrower1 transfers sUsdc to Lender1 with npv > 0", async function () {
        const sUsdcSupplyBefore = await sTokenTotalSupply(client, "USDC");
        const sUsdcBalanceBefore = await sTokenUnderlyingBalanceOf(client, "USDC");

        await transferStoken(client, "USDC", borrower1Keys, lender1Address, 10_000_000_000n);

        const sUsdcSupplyAfter = await sTokenTotalSupply(client, "USDC");
        const sUsdcBalanceAfter = await sTokenUnderlyingBalanceOf(client, "USDC");
        const lender1sUsdcBalance = await sTokenBalanceOf(client, "USDC", lender1Address);
        const borrower1sUsdcBalance = await sTokenBalanceOf(client, "USDC", borrower1Address);
        const borrower1PositionAfter = await accountPosition(client, borrower1Keys);

        assert.equal(lender1sUsdcBalance, 10_000_000_000n);
        assert.equal(borrower1sUsdcBalance, 90_000_000_000n);
        assert.equal(sUsdcSupplyBefore, sUsdcSupplyAfter);
        assert.equal(sUsdcBalanceBefore, sUsdcBalanceAfter);
        assert(borrower1PositionAfter.npv > 0);
    });

    it("Case 6: Borrower1 transfers sUsdc to Lender1 with npv <= 0", async function () {
        const sUsdcSupplyBefore = await sTokenTotalSupply(client, "USDC");
        const sUsdcBalanceBefore = await sTokenUnderlyingBalanceOf(client, "USDC");

        await transferStoken(client, "USDC", borrower1Keys, lender1Address, 10_000_000_000n);

        const sUsdcSupplyAfter = await sTokenTotalSupply(client, "USDC");
        const sUsdcBalanceAfter = await sTokenUnderlyingBalanceOf(client, "USDC");
        const lender1sUsdcBalance = await sTokenBalanceOf(client, "USDC", lender1Address);
        const borrower1sUsdcBalance = await sTokenBalanceOf(client, "USDC", borrower1Address);
        const borrower1PositionAfter = await accountPosition(client, borrower1Keys);

        assert.equal(lender1sUsdcBalance, 20_000_000_000n);
        assert.equal(borrower1sUsdcBalance, 80_000_000_000n);
        assert.equal(sUsdcSupplyBefore, sUsdcSupplyAfter);
        assert.equal(sUsdcBalanceBefore, sUsdcBalanceAfter);
        assert(borrower1PositionAfter.npv > 0);
    });
});