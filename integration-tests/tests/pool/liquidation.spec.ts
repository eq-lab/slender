import { SorobanClient } from "../soroban.client";
import { balanceOf, init, mintUnderlyingTo, registerAccount } from "../pool.sut";
import { adminKeys, borrower1Keys, lender1Keys } from "../soroban.config";
import {
    convertToScvAddress,
    convertToScvBool,
    convertToScvI128,
    parseMetaXdrToJs,
} from "../soroban.converter";
import { Keypair } from "soroban-client";

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
        let liquidatorKeys = Keypair.random();
        let liquidatorAddress = liquidatorKeys.publicKey();
        await registerAccount(client, "LIQUIDATOR", liquidatorKeys);
        console.log("liquidator", liquidatorAddress, liquidatorKeys.secret());
        console.log("borrower", borrower1Address, liquidatorKeys.secret());

        await mintUnderlyingTo(client, "XLM", lender1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "XRP", lender1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "USDC", lender1Address, 100_000_000_000n);

        await mintUnderlyingTo(client, "XRP", borrower1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "XLM", liquidatorAddress, 100_000_000_000n);
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

        console.log("deposit", JSON.stringify(lenderDepositResult, null, 2));

        const borrowerDepositResponse = await client.sendTransaction(
            process.env.SLENDER_POOL,
            "deposit",
            borrower1Keys,
            convertToScvAddress(borrower1Address),
            convertToScvAddress(process.env.SLENDER_TOKEN_XRP),
            convertToScvI128(10_000_000_000n)
        );

        const borrowerDepositResult = parseMetaXdrToJs(
            borrowerDepositResponse.resultMetaXdr
        );

        console.log("deposit", JSON.stringify(borrowerDepositResult, null, 2));

        const borrowerBorrowResponse = await client.sendTransaction(
            process.env.SLENDER_POOL,
            "borrow",
            borrower1Keys,
            convertToScvAddress(borrower1Address),
            convertToScvAddress(process.env.SLENDER_TOKEN_XLM),
            convertToScvI128(5_000_000_000n)
        );
        const borrowerBorrowResult = parseMetaXdrToJs(
            borrowerBorrowResponse.resultMetaXdr
        );
        console.log("borrow", JSON.stringify(borrowerBorrowResult, null, 2));

        const liquidatorDepositResponse = await client.sendTransaction(
            process.env.SLENDER_POOL,
            "deposit",
            liquidatorKeys,
            convertToScvAddress(liquidatorAddress),
            convertToScvAddress(process.env.SLENDER_TOKEN_XLM),
            convertToScvI128(10_000_000_000n)
        );
        const liquidatorDepositResult = parseMetaXdrToJs(
            liquidatorDepositResponse.resultMetaXdr
        );
        console.log("deposit", JSON.stringify(liquidatorDepositResult, null, 2));

        const liquidatorBorrowResponse = await client.sendTransaction(
            process.env.SLENDER_POOL,
            "borrow",
            liquidatorKeys,
            convertToScvAddress(liquidatorAddress),
            convertToScvAddress(process.env.SLENDER_TOKEN_XRP),
            convertToScvI128(5_000_000_000n)
        );
        const liquidatorBorrowResult = parseMetaXdrToJs(
            liquidatorBorrowResponse.resultMetaXdr
        );
        console.log("borrow", JSON.stringify(liquidatorBorrowResult, null, 2));

        const setPriceResponse = await client.sendTransaction(
            process.env.SLENDER_POOL,
            "set_price",
            adminKeys,
            convertToScvAddress(process.env.SLENDER_TOKEN_XLM),
            convertToScvI128(1_500_000_000n)
        );

        const setPriceResult = parseMetaXdrToJs(
            setPriceResponse.resultMetaXdr
        );

        console.log("set_price", JSON.stringify(setPriceResult, null, 2));

        const accountPositionResponse = await client.sendTransaction(
            process.env.SLENDER_POOL,
            "account_position",
            adminKeys,
            convertToScvAddress(borrower1Address),
        );

        const accountPositionResult = parseMetaXdrToJs(
            accountPositionResponse.resultMetaXdr
        );

        (BigInt.prototype as any).toJSON = function () {
            return this.toString(10);
          };
        console.log("account_position", JSON.stringify(accountPositionResult, null, 2))

        const liquidateResponse = await client.sendTransaction(
            process.env.SLENDER_POOL,
            "liquidate",
            liquidatorKeys,
            convertToScvAddress(liquidatorAddress),
            convertToScvAddress(borrower1Address),
            // convertToScvBool(true)
        );

        const liquidateResult = parseMetaXdrToJs(
            liquidateResponse.resultMetaXdr
        );

        console.log("liquidate", JSON.stringify(liquidateResult, null, 2));
    });
});
