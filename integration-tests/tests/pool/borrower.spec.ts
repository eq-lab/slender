import { SorobanClient } from "../soroban.client";
import { balanceOf, init, mintUnderlyingTo, registerAccount } from "../pool.sut";
import { adminKeys, borrower1Keys, lender1Keys } from "../soroban.config";
import {
    convertToScvAddress,
    convertToScvI128,
    parseMetaXdrToJs,
} from "../soroban.converter";

describe("LendingPool", function () {
    let client: SorobanClient;

    before(async function () {
        client = new SorobanClient();
        await init(client);
    });

    it("should TBD", async function () {
        // let lender1 = await registerAccount(client, "LENDER_1", lender1Keys);
        let lender1Address = lender1Keys.publicKey();
        let borrower1Address = borrower1Keys.publicKey();

        await mintUnderlyingTo(client, "XLM", lender1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "XRP", lender1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "USDC", lender1Address, 100_000_000_000n);

        await mintUnderlyingTo(client, "XRP", borrower1Address, 100_000_000_000n);
        // let lender1XlmBalance = await balanceOf(client, lender1Keys, lender1Address, "XLM");
        // let lender1XrpBalance = await balanceOf(client, lender1Keys, lender1Address, "XRP");
        // let lender1UsdcBalance = await balanceOf(client, lender1Keys, lender1Address, "USDC");

        const lenderDepositResponse = await client.sendTransaction(
            process.env.SLENDER_POOL,
            "deposit",
            lender1Keys,
            convertToScvAddress(lender1Address),
            convertToScvAddress(process.env.SLENDER_TOKEN_XLM),
            convertToScvI128(10000000000n)
        );
        const lenderDepositResult = parseMetaXdrToJs(
            lenderDepositResponse.resultMetaXdr
        );

        const borrowerDepositResponse = await client.sendTransaction(
            process.env.SLENDER_POOL,
            "deposit",
            borrower1Keys,
            convertToScvAddress(borrower1Address),
            convertToScvAddress(process.env.SLENDER_TOKEN_XRP),
            convertToScvI128(10000000000n)
        );
        const borrowerDepositResult = parseMetaXdrToJs(
            borrowerDepositResponse.resultMetaXdr
        );

        const borrowerBorrowResponse = await client.sendTransaction(
            process.env.SLENDER_POOL,
            "borrow",
            borrower1Keys,
            convertToScvAddress(borrower1Address),
            convertToScvAddress(process.env.SLENDER_TOKEN_XLM),
            convertToScvI128(10000000n)
        );
        const borrowerBorrowResult = parseMetaXdrToJs(
            borrowerBorrowResponse.resultMetaXdr
        );
        console.log(JSON.stringify(borrowerBorrowResult, null, 2));

        const lenderWithdrawResponse = await client.sendTransaction(
            process.env.SLENDER_POOL,
            "withdraw",
            lender1Keys,
            convertToScvAddress(lender1Address),
            convertToScvAddress(process.env.SLENDER_TOKEN_XLM),
            convertToScvI128(100000n),
            convertToScvAddress(lender1Address),
        );
        const lenderWithdrawResult = parseMetaXdrToJs(
            lenderWithdrawResponse.resultMetaXdr
        );
        console.log(JSON.stringify(lenderWithdrawResult, null, 2));
    });
});
