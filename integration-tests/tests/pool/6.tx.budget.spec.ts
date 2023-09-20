import { SorobanClient } from "../soroban.client";
import {
    borrow,
    cleanSlenderEnvKeys,
    deploy,
    deposit,
    init,
    mintUnderlyingTo,
    withdraw,
} from "../pool.sut";
import {
    borrower1Keys,
    borrower2Keys,
    lender1Keys,
} from "../soroban.config";
import { expect, use } from "chai";
import chaiAsPromised from 'chai-as-promised';
use(chaiAsPromised);

describe("LendingPool: methods must not exceed CPU/MEM limits", function () {
    let client: SorobanClient;
    let lender1Address: string;
    let borrower1Address: string;
    let borrower2Address: string;

    before(async function () {
        client = new SorobanClient();

        await cleanSlenderEnvKeys();
        await deploy();
        await init(client);

        lender1Address = lender1Keys.publicKey();
        borrower1Address = borrower1Keys.publicKey();
        borrower2Address = borrower2Keys.publicKey();

        await client.registerAccount(lender1Address);
        await client.registerAccount(borrower1Address);
        await client.registerAccount(borrower2Address);

        await mintUnderlyingTo(client, "XLM", lender1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "XRP", lender1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "USDC", lender1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "XLM", borrower1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "XRP", borrower1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "USDC", borrower2Address, 100_000_000_000n);

        // Lender1 deposits 10_000_000_000 XLM, XRP, USDC
        await deposit(client, lender1Keys, "XLM", 10_000_000_000n);
        await deposit(client, lender1Keys, "XRP", 10_000_000_000n);
        await deposit(client, lender1Keys, "USDC", 10_000_000_000n);

        // Borrower1 deposits 10_000_000_000 XLM, XRP, borrows 6_000_000_000 USDC
        await deposit(client, borrower1Keys, "XLM", 10_000_000_000n);
        await deposit(client, borrower1Keys, "XRP", 30_000_000_000n);
        await borrow(client, borrower1Keys, "USDC", 6_000_000_000n);

        // Borrower2 deposits 20_000_000_000 USDC, borrows 6_000_000_000 XLM, 5_999_000_000 XRP
        await deposit(client, borrower2Keys, "USDC", 20_000_000_000n);
        await borrow(client, borrower2Keys, "XLM", 6_000_000_000n);
        await borrow(client, borrower2Keys, "XRP", 5_900_000_000n);
    });

    it("Case 1: borrow()", async function () {
        // Borrower1 borrows 20_000_000 USDC
        await expect(borrow(client, borrower1Keys, "USDC", 20_000_000n)).to.not.eventually.rejected;
    });

    it("Case 2: withdraw full", async function () {
        // Borrower1 witdraws all XLM
        await expect(withdraw(client, borrower1Keys, "XLM", 170_141_183_460_469_231_731_687_303_715_884_105_727n)).to.not.eventually.rejected; // i128::MAX
    });
});
