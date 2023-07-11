const SorobanClient = require("../test-fixture");

describe("LendingPool", function () {
    let client;

    before(async function () {
        client = new SorobanClient();
    });

    beforeEach(async function () {
    });

    describe("init()", function () {
        it("should correctly initialize the state after deployment", async function () {
            const result = await client.sendTransaction(
                process.env.PRICE_FEED,
                "decimals",
                process.env.POOL_PUBLIC,
                process.env.POOL_SECRET
            );

            // const qweqweqwe = Client.xdr.TransactionResult
            //     .fromXDR(result.resultXdr, "base64")
            //     .result()
            //     .results()[0]
            //     .tr()
            //     .invokeHostFunctionResult()
            //     .success()[0]
            //     .value();
        });
    });
});
