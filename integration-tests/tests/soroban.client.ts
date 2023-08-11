import {
  Server,
  Contract,
  TimeoutInfinite,
  TransactionBuilder,
  Keypair,
  xdr,
  SorobanRpc,
  Account,
} from "soroban-client";
import { promisify } from "util";
import "./soroban.config";
import { Durability } from "soroban-client/lib/server";

export class SorobanClient {
  client: Server;
  unlimitedResources: boolean;

  constructor() {
    this.client = new Server(process.env.SOROBAN_RPC_URL, {
      allowHttp: true,
    });
    this.client.getHealth();
  }

  setUnlimitedResources() {
    this.unlimitedResources = true;
  }

  async registerAccount(publicKey: string): Promise<Account> {
    return await this.client.requestAirdrop(
      publicKey,
      process.env.FRIENDBOT_URL
    );
  }

  async sendTransaction(
    contractId: string,
    method: string,
    signer: Keypair,
    ...args: xdr.ScVal[]
  ): Promise<SorobanRpc.GetTransactionResponse> {
    const source = await this.client.getAccount(signer.publicKey());
    const contract = new Contract(contractId);

    const contractData = await this.client.getContractData(
      contract,
      xdr.ScVal.scvLedgerKeyContractInstance(),
      Durability.Persistent
    );

    const entry = xdr.LedgerEntryData.fromXDR(contractData.xdr, "base64");
    // @ts-ignore
    const instance = new xdr.ScContractInstance({
      // @ts-ignore
      executable: entry.contractData().body().value().val(),
    });
    // @ts-ignore
    const executable = xdr.ContractExecutable.contractExecutableWasm(
      // @ts-ignore
      instance.executable()
    );
    // @ts-ignore
    const hash = executable.wasmHash().instance().executable().wasmHash();

    const txBuilder = new TransactionBuilder(source, {
      fee: "100000",
      networkPassphrase: process.env.PASSPHRASE,
    })
      .addOperation(contract.call(method, ...(args || [])))
      .setTimeout(TimeoutInfinite);

    if (this.unlimitedResources) {
      const bodyType = xdr.ContractEntryBodyType.dataEntry();
      const durability = xdr.ContractDataDurability.persistent();

      // @ts-ignore
      const extPoint = new xdr.ExtensionPoint(0);

      const transactionData = new xdr.SorobanTransactionData({
        ext: extPoint,
        resources: new xdr.SorobanResources({
          footprint: new xdr.LedgerFootprint({
            readOnly: [
              xdr.LedgerKey.contractData(
                new xdr.LedgerKeyContractData({
                  contract: contract.address().toScAddress(),
                  key: xdr.ScVal.scvLedgerKeyContractInstance(),
                  durability,
                  bodyType,
                })
              ),
              xdr.LedgerKey.contractCode(
                new xdr.LedgerKeyContractCode({
                  hash,
                  bodyType,
                })
              ),
            ],
            readWrite: [
              // xdr.LedgerKey.contractData(
              //   new xdr.LedgerKeyContractData({
              //     contract: contract.address().toScAddress(),
              //     key: xdr.ScVal.scvVec([
              //       xdr.ScVal.scvSymbol("Balance"),
              //       args[0],
              //     ]),
              //     durability,
              //     bodyType,
              //   })
              // ),

              xdr.LedgerKey.contractData(
                new xdr.LedgerKeyContractData({
                  contract: contract.address().toScAddress(),
                  key: xdr.ScVal.scvVec([
                    xdr.ScVal.scvSymbol("UserConfig"),
                    args[0],
                  ]),
                  durability,
                  bodyType,
                })
              ),

              xdr.LedgerKey.contractData(
                new xdr.LedgerKeyContractData({
                  contract: contract.address().toScAddress(),
                  key: xdr.ScVal.scvVec([
                    xdr.ScVal.scvSymbol("ReserveAssetKey"),
                    args[1],
                  ]),
                  durability,
                  bodyType,
                })
              ),

              xdr.LedgerKey.contractData(
                new xdr.LedgerKeyContractData({
                  contract: contract.address().toScAddress(),
                  key: xdr.ScVal.scvVec([xdr.ScVal.scvSymbol("Reserves")]),
                  durability,
                  bodyType,
                })
              ),

              xdr.LedgerKey.contractData(
                new xdr.LedgerKeyContractData({
                  contract: contract.address().toScAddress(),
                  key: xdr.ScVal.scvVec([xdr.ScVal.scvSymbol("IRParams")]),
                  durability,
                  bodyType,
                })
              ),

              xdr.LedgerKey.contractData(
                new xdr.LedgerKeyContractData({
                  contract: contract.address().toScAddress(),
                  key: xdr.ScVal.scvVec([xdr.ScVal.scvSymbol("Pause")]),
                  durability,
                  bodyType,
                })
              ),
            ],
          }),
          instructions: 100_000_000,
          readBytes: 100_000_000,
          writeBytes: 100_000_000,
          extendedMetaDataSizeBytes: 204_800,
        }),
        refundableFee: xdr.Int64.fromString("204800"),
      });

      txBuilder.setSorobanData(transactionData);
    }

    const transaction = await this.client.prepareTransaction(
      txBuilder.build(),
      process.env.PASSPHRASE
    );

    transaction.sign(signer);

    const response = await this.client.sendTransaction(transaction);

    let result: SorobanRpc.GetTransactionResponse;
    let attempts = 10;

    do {
      await delay(1000);
      result = await this.client.getTransaction(response.hash);
      attempts--;
    } while (result.status === "NOT_FOUND" && attempts > 0);

    return result;
  }
}

let delay = promisify((ms, res) => setTimeout(res, ms));
