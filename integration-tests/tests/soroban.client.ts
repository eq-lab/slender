import { Server, Contract, TimeoutInfinite, TransactionBuilder, Keypair, xdr, SorobanRpc, Account } from "soroban-client";
import { promisify } from "util";
import "./soroban.config";
import { convertToScvAddress } from "./soroban.converter";

export class SorobanClient {
    client: Server;

    constructor() {
        this.client = new Server(process.env.SOROBAN_RPC_URL, {
            allowHttp: true
        });
        this.client.getHealth();
    }

    async registerAccount(publicKey: string): Promise<Account> {
        return await this.client.requestAirdrop(publicKey, process.env.FRIENDBOT_URL);
    }

    async sendTransaction(
        contractId: string,
        method: string,
        signer: Keypair,
        ...args: xdr.ScVal[]
    ): Promise<SorobanRpc.GetTransactionResponse> {
        const source = await this.client.getAccount(signer.publicKey());
        const contract = new Contract(contractId);

        const operation = new TransactionBuilder(source, {
            fee: "100",
            networkPassphrase: process.env.PASSPHRASE,
        }).addOperation(contract.call(method, ...args || []))
            .setTimeout(TimeoutInfinite)
            .build();

        const transaction = await this.client.prepareTransaction(
            operation,
            process.env.PASSPHRASE);

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

let delay = promisify((ms, res) => setTimeout(res, ms))
