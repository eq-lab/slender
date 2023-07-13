import { Server, Contract, TimeoutInfinite, TransactionBuilder, Keypair, xdr, SorobanRpc } from "soroban-client";
import "./soroban.config";

export class SorobanClient {
    client: Server;

    constructor() {
        this.client = new Server(process.env.SOROBAN_RPC_URL, {
            allowHttp: true
        });
        this.client.getHealth();
    }

    async sendTransaction(
        contractId: string,
        method: string,
        signer: string,
        secret: string,
        ...args: xdr.ScVal[]
    ): Promise<SorobanRpc.GetTransactionResponse> {
        const source = await this.client.getAccount(signer);
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

        transaction.sign(Keypair.fromSecret(secret));

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

async function delay(ms: number): Promise<void> {
    await new Promise(res => setTimeout(res, ms));
}
