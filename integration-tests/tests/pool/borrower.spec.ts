import { SorobanClient } from "../soroban.client";
import { balanceOf, init, mintUnderlyingTo, registerAccount } from "../pool.sut";
import { adminKeys, lender1Keys } from "../soroban.config";
import { convertToScvAddress, convertToScvI128, parseMetaXdrToJs } from "../soroban.converter";

describe("LendingPool", function () {
    let client: SorobanClient;

    before(async function () {
        client = new SorobanClient();
        await init(client);
    });

    it("should TBD", async function () {
        // let lender1 = await registerAccount(client, "LENDER_1", lender1Keys);
        let lender1Address = lender1Keys.publicKey();

        await mintUnderlyingTo(client, "XLM", lender1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "XRP", lender1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "USDC", lender1Address, 100_000_000_000n);
        // let lender1XlmBalance = await balanceOf(client, lender1Keys, lender1Address, "XLM");
        // let lender1XrpBalance = await balanceOf(client, lender1Keys, lender1Address, "XRP");
        // let lender1UsdcBalance = await balanceOf(client, lender1Keys, lender1Address, "USDC");

        const depositResponse = await client.sendTransaction(
            process.env.SLENDER_POOL,
            "deposit",
            lender1Keys,
            convertToScvAddress(lender1Address),
            convertToScvAddress(process.env.SLENDER_TOKEN_XLM),
            convertToScvI128(10000000000n)
        );

        const depositResult = parseMetaXdrToJs(depositResponse.resultMetaXdr);
    });
});
