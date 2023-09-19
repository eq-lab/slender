import { Server, Contract, TimeoutInfinite, TransactionBuilder, Keypair, xdr, SorobanRpc } from "soroban-client";
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
    ): Promise<SorobanRpc.GetSuccessfulTransactionResponse> {
        const source = await this.client.getAccount(signer.publicKey());
        const contract = new Contract(contractId);

        const operation = new TransactionBuilder(source, {
            fee: "100000000",
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
        let attempts = 15;

        if (response.status == "ERROR") {
            throw Error(`ERROR [sendTransaction]: ${response.errorResultXdr}`);
        }

        do {
            await delay(1000);
            result = await this.client.getTransaction(response.hash);
            attempts--;
        } while (result.status === SorobanRpc.GetTransactionStatus.NOT_FOUND && attempts > 0);

        if (result.status == SorobanRpc.GetTransactionStatus.NOT_FOUND) {
            console.error(`NOT_FOUND [getTransaction]: ${JSON.stringify(response, null, 2)}`);
        }

        if ("resultXdr" in result) {
            const getResult = result as SorobanRpc.GetTransactionResponse;
            if (getResult.status !== SorobanRpc.GetTransactionStatus.SUCCESS) {
                console.error('Transaction submission failed! Returning full RPC response.');
                return result;
            }

            return result;
        }

        throw Error(`Transaction failed (method: ${method})`);
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

        const simulated = await this.client.simulateTransaction(operation);

        if (SorobanRpc.isSimulationError(simulated)) {
            throw new Error(simulated.error);
        } else if (!simulated.result) {
            throw new Error(`invalid simulation: no result in ${simulated}`);
        }

        return simulated.result.retval;
    }
}

let delay = promisify((ms, res) => setTimeout(res, ms))
