import { SorobanClient } from "../soroban.client";
import { init } from "../pool.sut";
import { parseScvToJs } from "../soroban.converter";
import { Contract, xdr } from "soroban-client";

describe("LendingPool", function () {
    let client: SorobanClient;

    before(async function () {
        client = new SorobanClient();
        await init(client);
    });

    it("should TBD", async function () {
        let ledgerKey = xdr.LedgerKey.contractData(
            new xdr.LedgerKeyContractData({
                contract: new Contract("CBNGM7ZDA6PJSFLJI3VDHNPUBGUNQIUMPEA7XJ37PMWTMJAP4WRQNNEZ").address().toScAddress(),
                key: xdr.ScVal.scvLedgerKeyContractInstance(),
                durability: xdr.ContractDataDurability.persistent(),
                bodyType: xdr.ContractEntryBodyType.dataEntry(),
            })
        );
        let poolInstanceLedgerEntriesRaw = await client.client.getLedgerEntries([ledgerKey]);
        const poolInstanceLedgerEntries = xdr.LedgerEntryData
            .fromXDR(poolInstanceLedgerEntriesRaw.entries[0].xdr, "base64");
        const poolInstanceStorageEntries = (poolInstanceLedgerEntries.value() as any).body().value().val().value().storage();

        // ir params
        const vec_key_1 = parseScvToJs(poolInstanceStorageEntries[1].key());
        const vec_value_1 = parseScvToJs(poolInstanceStorageEntries[1].val());
        console.log(`KEY: ${vec_key_1} \nVALUE: ${JSON.stringify(vec_value_1, null, 2)}`);

        for (let i = 0; i < poolInstanceStorageEntries.length; i++) {
            console.log(`KEY ${i + 1}: ${parseScvToJs(poolInstanceStorageEntries[i].key())}`);
        }
    });
});
