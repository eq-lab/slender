import { Server, Contract, TimeoutInfinite, TransactionBuilder, Keypair, xdr, SorobanRpc, BASE_FEE, assembleTransaction } from "soroban-client";
import { promisify } from "util";
import "./soroban.config";
import { adminKeys } from "./soroban.config";

export class SendTransactionResult {
    response: SorobanRpc.GetSuccessfulTransactionResponse;
    simulation?: SorobanRpc.SimulateTransactionSuccessResponse

    constructor(response: SorobanRpc.GetSuccessfulTransactionResponse, simulation?: SorobanRpc.SimulateTransactionSuccessResponse) {
        this.response = response;
        this.simulation = simulation;
    }
}

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
        retryAttempts: number,
        ...args: xdr.ScVal[]
    ): Promise<SendTransactionResult> {
        const source = await this.client.getAccount(signer.publicKey());
        const contract = new Contract(contractId);

        const operation = new TransactionBuilder(source, {
            fee: BASE_FEE,
            networkPassphrase: process.env.PASSPHRASE,
        }).addOperation(contract.call(method, ...args || []))
            .setTimeout(TimeoutInfinite)
            .build();

        const simulated = await this.client.simulateTransaction(operation) as SorobanRpc.SimulateTransactionSuccessResponse;

        if (SorobanRpc.isSimulationError(simulated)) {
            throw new Error(simulated.error);
        } else if (!simulated.result) {
            throw new Error(`Invalid simulation: no result in ${simulated}`);
        }

        const transaction = assembleTransaction(operation, process.env.PASSPHRASE, simulated).build()

        transaction.sign(signer);

        const response = await this.client.sendTransaction(transaction);

        let result: SorobanRpc.GetTransactionResponse;
        let attempts = 15;

        if (response.status == "ERROR") {
            throw Error(`Failed to send transaction: ${response.errorResult.toXDR("base64")}`);
        }

        do {
            await delay(1000);
            result = await this.client.getTransaction(response.hash);
            attempts--;
        } while (result.status === SorobanRpc.GetTransactionStatus.NOT_FOUND && attempts > 0);

        if (result.status == SorobanRpc.GetTransactionStatus.NOT_FOUND) {
            throw Error("Submitted transaction was not found");
        }

        if ("resultXdr" in result) {
            const getResult = result as SorobanRpc.GetTransactionResponse;
            if (getResult.status !== SorobanRpc.GetTransactionStatus.SUCCESS) {
                throw new Error('Transaction result is insuccessfull');
            }

            console.log(`    SUCCESS: '${method}' => ${signer.publicKey()}`);

            return new SendTransactionResult(result, simulated);
        }

        if (retryAttempts == 0) {
            throw Error(`Transaction failed (method: ${method})`);
        } else {
            return await this.sendTransaction(
                contractId,
                method,
                signer,
                --retryAttempts,
                ...args || []
            );
        }
    }

    async simulateTransaction(
        contractId: string,
        method: string,
        ...args: xdr.ScVal[]
    ): Promise<xdr.ScVal> {
        const source = await this.client.getAccount(adminKeys.publicKey());
        const contract = new Contract(contractId);

        const operation = new TransactionBuilder(source, {
            fee: BASE_FEE,
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

export let delay = promisify((ms, res) => setTimeout(res, ms))
