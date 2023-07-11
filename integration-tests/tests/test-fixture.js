const Client = require("soroban-client");
require('dotenv').config({ path: "integration-tests/.env" });
//"scripts": {
//     "test-dev": "NODE_ENV=development nodemon index.js",
//     "test-prod": "NODE_ENV=production node index.js"
// }
//    { path: `.env.${process.env.NODE_ENV}` }
require("dotenv").config({ path: "integration-tests/.contracts" });
// TODO: readme.md

module.exports = class SorobanClient {
    constructor() {
        this.client = new Client.Server(process.env.SOROBAN_RPC_URL, {
            allowHttp: true
        });
    }

    async sendTransaction(
        contractId,
        method,
        signer,
        secret,
        args
    ) {
        const source = await this.client.getAccount(signer);
        const contract = new Client.Contract(contractId);

        const operation = new Client
            .TransactionBuilder(source, {
                fee: 100,
                networkPassphrase: process.env.PASSPHRASE,
            })
            .addOperation(contract.call(method, ...args || []))
            .setTimeout(Client.TimeoutInfinite)
            .build();

        const transaction = await this.client.prepareTransaction(
            operation,
            process.env.PASSPHRASE);

        transaction.sign(Client.Keypair.fromSecret(secret));

        const response = await this.client.sendTransaction(transaction);

        let result;
        let attempts = 10;

        do {
            await this.delay(3000);
            result = await this.client.getTransaction(response.hash);
            attempts--;
        } while (result.status === "NOT_FOUND" && attempts > 0);

        return result;
    }

    async delay(ms) {
        await new Promise(res => setTimeout(res, ms));
    }
}