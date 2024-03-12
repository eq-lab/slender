import { SorobanClient, delay } from "../soroban.client";
import {
    FlashLoanAsset,
    cleanSlenderEnvKeys,
    debtTokenBalanceOf,
    debtTokenTotalSupply,
    deploy,
    deployReceiverMock,
    deposit,
    flashLoan,
    init,
    initializeFlashLoanReceiver,
    mintUnderlyingTo,
    sTokenUnderlyingBalanceOf,
    tokenBalanceOf,
} from "../pool.sut";
import {
    adminKeys,
    borrower1Keys,
    lender1Keys,
    treasuryKeys,
} from "../soroban.config";
import { assert, expect, use } from "chai";
import chaiAsPromised from 'chai-as-promised';
use(chaiAsPromised);

describe("LendingPool: Borrower makes a call to the flash loan with custom receiver contract", function () {
    let client: SorobanClient;
    let treasuryAddress: string;
    let borrower1Address: string;
    let lender1Address: string;
    let receiverAddress: string;
    let failingReceiverAddress: string;

    before(async function () {
        client = new SorobanClient();

        await cleanSlenderEnvKeys();
        await deploy();
        await init(client);

        treasuryAddress = treasuryKeys.publicKey();
        borrower1Address = borrower1Keys.publicKey();
        lender1Address = lender1Keys.publicKey();

        // uncomment to resume test with existing contracts
        // require("dotenv").config({ path: contractsFilename });
        // return;

        await Promise.all([
            client.registerAccount(lender1Address),
            client.registerAccount(borrower1Address),
        ]);

        await mintUnderlyingTo(client, "XLM", lender1Address, 1_000_000_000n);
        await mintUnderlyingTo(client, "XLM", borrower1Address, 1_000_000_000n);
        await mintUnderlyingTo(client, "XRP", borrower1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "USDC", borrower1Address, 100_000_000_000n);

        receiverAddress = await deployReceiverMock();
        failingReceiverAddress = await deployReceiverMock();

        await initializeFlashLoanReceiver(client, adminKeys, receiverAddress, false);
        await initializeFlashLoanReceiver(client, adminKeys, failingReceiverAddress, true);

        await deposit(client, lender1Keys, "XLM", 500_000_000n);
    });

    it("Case 1: Borrower borrows without opening debt position", async function () {
        const loanAssets: FlashLoanAsset[] = [
            {
                asset: "XLM",
                amount: 10_000_000n,
                borrow: false
            }
        ];

        await flashLoan(client, borrower1Keys, receiverAddress, loanAssets, "00");

        const treasuryXlmBalance = await tokenBalanceOf(client, "XLM", treasuryAddress);
        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");

        assert.equal(treasuryXlmBalance, 5_000n);
        assert.equal(borrower1XlmBalance, 999_995_000n);
        assert.equal(borrower1DXlmBalance, 0n);
        assert.equal(sXlmBalance, 500_000_000n);
        assert.equal(dXlmSupply, 0n);
    });

    // TODO: requires increasing CPU limits
    // it("Case 2: Borrower borrows with opening debt position", async function () {
    //     await deposit(client, borrower1Keys, "USDC", 10_000_000_000n);

    //     const loanAssets: FlashLoanAsset[] = [
    //         {
    //             asset: "XLM",
    //             amount: 10_000_000n,
    //             borrow: true
    //         }
    //     ];

    //     await flashLoan(client, borrower1Keys, receiverAddress, loanAssets, "00");
    // });

    // TODO: requires increasing CPU limits
    // it("Case 3: Borrower borrows with opening debt position when not enough collateral", async function () {
    //     const flashLoanReceiverMock = await deployReceiverMock();

    //     const loanAssets: FlashLoanAsset[] = [
    //         {
    //             asset: "XLM",
    //             amount: 8_000_000n,
    //             borrow: true
    //         }
    //     ];

    //     await flashLoan(client, borrower1Keys, flashLoanReceiverMock, loanAssets, "00");
    // });

    it("Case 4: Receiver returns failed status (false)", async function () {
        const loanAssets: FlashLoanAsset[] = [
            {
                asset: "XLM",
                amount: 10_000_000n,
                borrow: false
            }
        ];

        await expect(
            flashLoan(client, borrower1Keys, failingReceiverAddress, loanAssets, "00")
        ).to.eventually.rejected;
    });
});
