import { Server, Contract, TimeoutInfinite, TransactionBuilder, Keypair, xdr, SorobanRpc, Account } from "soroban-client";
import { promisify } from "util";
import "./soroban.config";
import { adminKeys } from "./soroban.config";

export class SorobanClient {
    client: Server;

    constructor() {
        this.client = new Server(process.env.SOROBAN_RPC_URL, {
            allowHttp: true
        });
        this.client.getHealth();
    }

    async registerAccount(publicKey: string): Promise<void> {
        await this.client
            .requestAirdrop(publicKey, process.env.FRIENDBOT_URL)
            .catch(_ => { });
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

        console.log(`${signer.publicKey()} => ${method} => ${result.status}`);

        return result;
    }

    async simulateTransaction(
        contractId: string,
        method: string,
        ...args: xdr.ScVal[]
    ): Promise<xdr.ScVal> {
        const source = await this.client.getAccount(adminKeys.publicKey());
        const contract = new Contract(contractId);

        const operation = new TransactionBuilder(source, {
            fee: "100",
            networkPassphrase: process.env.PASSPHRASE,
        }).addOperation(contract.call(method, ...args || []))
            .setTimeout(TimeoutInfinite)
            .build();

        const { results } = await this.client.simulateTransaction(operation);
        if (!results || results.length !== 1) {
            throw new Error("Invalid response from simulateTransaction");
        }

        return xdr.ScVal.fromXDR(results[0].xdr, "base64");
    }
}

let delay = promisify((ms, res) => setTimeout(res, ms))
