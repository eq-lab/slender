import { SorobanClient } from "../soroban.client";
import {
  balanceOf,
  init,
  mintUnderlyingTo,
  registerAccount,
} from "../pool.sut";
import { lender1Keys } from "../soroban.config";

describe("LendingPool", function () {
  let client: SorobanClient;

  before(async function () {
    client = new SorobanClient();
    await init(client);
  });

  it("should TBD", async function () {
    try {
      await registerAccount(client, "LENDER_1", lender1Keys);
    } catch {}
    let lender1Address = lender1Keys.publicKey();

    await mintUnderlyingTo(client, "XLM", lender1Address, 100_000_000_000n);
    await mintUnderlyingTo(client, "XRP", lender1Address, 100_000_000_000n);
    await mintUnderlyingTo(client, "USDC", lender1Address, 100_000_000_000n);
    let lender1XlmBalance = await balanceOf(
      client,
      lender1Keys,
      lender1Address,
      "XLM"
    );
    let lender1XrpBalance = await balanceOf(
      client,
      lender1Keys,
      lender1Address,
      "XRP"
    );

    client.setUnlimitedResources();
    let lender1UsdcBalance = await balanceOf(
      client,
      lender1Keys,
      lender1Address,
      "USDC"
    );

    //     const debtTokenResult = await this.client.sendTransaction(
    //         process.env.TOKEN,
    //         "mint",
    //         tokenKeys,
    //         addressToScVal(process.env.USER_PUBLIC),
    //         i128ToScVal(BigInt(10000000000n))
    //     );

    //     expect(debtTokenResult.status).to.equal("SUCCESS");

    //     const userBalanceResult = await this.client.sendTransaction(
    //         process.env.TOKEN,
    //         "balance",
    //         tokenKeys,
    //         addressToScVal(process.env.USER_PUBLIC)
    //     );

    //     const minted = parseScVal(userBalanceResult.resultXdr);

    //     expect(userBalanceResult.status).to.equal("SUCCESS");
    //     expect(minted).to.equal(10000000000n);
  });
});
