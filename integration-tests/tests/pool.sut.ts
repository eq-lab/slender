import { Keypair } from "soroban-client";
import { SendTransactionResult, SorobanClient } from "./soroban.client";
import { adminKeys, contractsFilename, setEnv, treasuryKeys } from "./soroban.config";
import {
    convertToScvAddress,
    convertToScvVec,
    convertToScvBool,
    convertToScvI128,
    convertToScvU32,
    convertToScvMap,
    parseMetaXdrToJs,
    convertToScvBytes,
    convertToScvString,
    parseScvToJs
} from "./soroban.converter";
import { exec } from "child_process";
import * as fs from 'fs';

export const BUDGET_SNAPSHOT_FILE = 'snapshots/budget_utilization.snap';

export type SlenderAsset = "XLM" | "XRP" | "USDC";

export interface AccountPosition {
    debt: bigint;
    discounted_collateral: bigint;
    npv: bigint;
}

export interface FlashLoanAsset {
    asset: SlenderAsset,
    amount: bigint,
    borrow: boolean
}

export async function init(client: SorobanClient): Promise<void> {
    console.log("    Contracts initialization has been started");

    require("dotenv").config({ path: contractsFilename });

    let salt = 0;
    const generateSalt = (value: number): string => String(value).padStart(64, '0');

    await initToken(client, "XLM", "Lumens");
    await initToken(client, "XRP", "Ripple");
    await initToken(client, "USDC", "USD Coin");

    await initPool(client, `${generateSalt(++salt)}`);

    await initSToken(client, "XLM", `${generateSalt(++salt)}`);
    await initSToken(client, "XRP", `${generateSalt(++salt)}`);
    await initSToken(client, "USDC", `${generateSalt(++salt)}`);

    await initDToken(client, "XLM", `${generateSalt(++salt)}`);
    await initDToken(client, "XRP", `${generateSalt(++salt)}`);
    await initDToken(client, "USDC", `${generateSalt(++salt)}`);

    await initPoolReserve(client, "XLM", 9);
    await initPoolReserve(client, "XRP", 9);
    await initPoolReserve(client, "USDC", 9);

    await initPoolCollateral(client, "XLM");
    await initPoolCollateral(client, "XRP");
    await initPoolCollateral(client, "USDC");

    await initPoolBorrowing(client, "XLM");
    await initPoolBorrowing(client, "XRP");
    await initPoolBorrowing(client, "USDC");

    await initBaseAsset(client, "XLM");

    await initPoolPriceFeed(client, process.env.SLENDER_PRICE_FEED, ["XRP", "USDC"]);

    console.log("    Contracts initialization has been finished");
}

export async function mintUnderlyingTo(
    client: SorobanClient,
    asset: SlenderAsset,
    to: string,
    amount: bigint
): Promise<void> {
    await initContract(
        `${to}_${asset}_MINTED`,
        () => client.sendTransaction(
            process.env[`SLENDER_TOKEN_${asset}`],
            "mint",
            adminKeys,
            convertToScvAddress(to),
            convertToScvI128(amount)
        )
    );
}

export async function sTokenBalanceOf(
    client: SorobanClient,
    asset: SlenderAsset,
    address: string
): Promise<bigint> {
    const xdrResponse = await client.simulateTransaction(
        process.env[`SLENDER_S_TOKEN_${asset}`],
        "balance",
        convertToScvAddress(address)
    );

    return parseScvToJs(xdrResponse);
}

export async function debtTokenBalanceOf(
    client: SorobanClient,
    asset: SlenderAsset,
    address: string
): Promise<bigint> {
    const xdrResponse = await client.simulateTransaction(
        process.env[`SLENDER_DEBT_TOKEN_${asset}`],
        "balance",
        convertToScvAddress(address)
    );

    return parseScvToJs(xdrResponse);
}

export async function tokenBalanceOf(
    client: SorobanClient,
    asset: SlenderAsset,
    address: string
): Promise<bigint> {
    const xdrResponse = await client.simulateTransaction(
        process.env[`SLENDER_TOKEN_${asset}`],
        "balance",
        convertToScvAddress(address)
    );

    return parseScvToJs(xdrResponse);
}

export async function accountPosition(
    client: SorobanClient,
    signer: Keypair,
): Promise<AccountPosition> {
    const xdrResponse = await client.simulateTransaction(
        process.env.SLENDER_POOL,
        "account_position",
        convertToScvAddress(signer.publicKey())
    );

    return parseScvToJs<AccountPosition>(xdrResponse);
}

export async function setPrice(
    client: SorobanClient,
    asset: SlenderAsset,
    amount: bigint,
): Promise<SendTransactionResult> {
    return client.sendTransaction(
        process.env.SLENDER_PRICE_FEED,
        "set_price",
        adminKeys,
        convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
        convertToScvI128(amount)
    );
}

export async function sTokenUnderlyingBalanceOf(
    client: SorobanClient,
    asset: SlenderAsset,
): Promise<bigint> {
    const xdrResponse = await client.simulateTransaction(
        process.env.SLENDER_POOL,
        "stoken_underlying_balance",
        convertToScvAddress(process.env[`SLENDER_S_TOKEN_${asset}`])
    );

    return parseScvToJs(xdrResponse);
}

export async function sTokenTotalSupply(
    client: SorobanClient,
    asset: SlenderAsset,
): Promise<bigint> {
    const xdrResponse = await client.simulateTransaction(
        process.env[`SLENDER_S_TOKEN_${asset}`],
        "total_supply",
    );

    return parseScvToJs(xdrResponse);
}

export async function debtTokenTotalSupply(
    client: SorobanClient,
    asset: SlenderAsset,
): Promise<bigint> {
    const xdrResponse = await client.simulateTransaction(
        process.env[`SLENDER_DEBT_TOKEN_${asset}`],
        "total_supply",
    );

    return parseScvToJs(xdrResponse);
}

export async function borrow(
    client: SorobanClient,
    signer: Keypair,
    asset: SlenderAsset,
    amount: bigint,
): Promise<SendTransactionResult> {
    const txResult = await client.sendTransaction(
        process.env.SLENDER_POOL,
        "borrow",
        signer,
        convertToScvAddress(signer.publicKey()),
        convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
        convertToScvI128(amount)
    );

    return txResult;
}

export async function deposit(
    client: SorobanClient,
    signer: Keypair,
    asset: SlenderAsset,
    amount: bigint,
): Promise<SendTransactionResult> {
    const txResult = await client.sendTransaction(
        process.env.SLENDER_POOL,
        "deposit",
        signer,
        convertToScvAddress(signer.publicKey()),
        convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
        convertToScvI128(amount)
    );

    return txResult;
}

export async function repay(
    client: SorobanClient,
    signer: Keypair,
    asset: SlenderAsset,
    amount: bigint,
): Promise<SendTransactionResult> {
    const txResult = await client.sendTransaction(
        process.env.SLENDER_POOL,
        "repay",
        signer,
        convertToScvAddress(signer.publicKey()),
        convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
        convertToScvI128(amount)
    );

    return txResult;
}

export async function withdraw(
    client: SorobanClient,
    signer: Keypair,
    asset: SlenderAsset,
    amount: bigint,
): Promise<SendTransactionResult> {
    const txResult = await client.sendTransaction(
        process.env.SLENDER_POOL,
        "withdraw",
        signer,
        convertToScvAddress(signer.publicKey()),
        convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
        convertToScvI128(amount),
        convertToScvAddress(signer.publicKey())
    );

    return txResult;
}

export async function liquidate(
    client: SorobanClient,
    signer: Keypair,
    who: string,
    receiveStoken: boolean,
): Promise<SendTransactionResult> {
    const txResult = await client.sendTransaction(
        process.env.SLENDER_POOL,
        "liquidate",
        signer,
        convertToScvAddress(signer.publicKey()),
        convertToScvAddress(who),
        convertToScvBool(receiveStoken)
    );

    return txResult;
}

export async function collatCoeff(
    client: SorobanClient,
    asset: SlenderAsset
): Promise<bigint> {
    const xdrResponse = await client.simulateTransaction(
        process.env.SLENDER_POOL,
        "collat_coeff",
        convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`])
    );

    return parseScvToJs<bigint>(xdrResponse);
}

export async function transferStoken(
    client: SorobanClient,
    asset: SlenderAsset,
    signer: Keypair,
    to: string,
    amount: bigint,
): Promise<SendTransactionResult> {
    return client.sendTransaction(
        process.env[`SLENDER_S_TOKEN_${asset}`],
        "transfer",
        signer,
        convertToScvAddress(signer.publicKey()),
        convertToScvAddress(to),
        convertToScvI128(amount)
    );
}

export async function deploy(): Promise<void> {
    console.log("    Contracts deployment has been started");

    const stdout = await new Promise((resolve, reject) => {
        exec(`../deploy/scripts/deploy.sh ${process.env.NODE_ENV}`, (error, stdout, _) => {
            if (error) {
                reject(error);
                return;
            }
            resolve(stdout)
        });
    });
    console.log(stdout);
    console.log("    Contracts deployment has been finished");
}

export async function deployReceiverMock(): Promise<string> {
    console.log("    Flashloan receiver deployment has been started");
    const flashLoadReceiverMockAddress = (await new Promise((resolve, reject) => {
        exec(`soroban contract deploy \
        --wasm ../target/wasm32-unknown-unknown/release/flash_loan_receiver_mock.wasm \
        --source ${adminKeys.secret()} \
        --rpc-url "${process.env.SOROBAN_RPC_URL}" \
        --network-passphrase "${process.env.PASSPHRASE}"`, (error, stdout, _) => {
            if (error) {
                reject(error);
                return;
            }
            resolve(stdout)
        });
    }) as string).trim();
    setEnv("SLENDER_FLASHLOAN_RECEIVER_MOCK", flashLoadReceiverMockAddress);
    console.log("    Flashloan receiver deployment has been finished");
    return (flashLoadReceiverMockAddress as string).trim();
}

export async function cleanSlenderEnvKeys() {
    Object.keys(process.env).forEach(key => {
        if (key.startsWith("SLENDER_")) {
            delete process.env[key];
        }
    });
}

export async function finalizeTransfer(
    client: SorobanClient,
    asset: SlenderAsset,
    signer: Keypair,
    to: string,
    amount: bigint,
    from_before: bigint,
    to_before: bigint,
    total_supply: bigint
) {
    await client.sendTransaction(
        process.env["SLENDER_POOL"],
        "finalize_transfer",
        signer,
        convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
        convertToScvAddress(signer.publicKey()),
        convertToScvAddress(to),
        convertToScvI128(amount),
        convertToScvI128(from_before),
        convertToScvI128(to_before),
        convertToScvI128(total_supply),
    );
}

export async function flashLoan(
    client: SorobanClient,
    signer: Keypair,
    receiver: string,
    loanAssets: FlashLoanAsset[],
    params: string
): Promise<SendTransactionResult> {
    const toConvert = loanAssets.map((flashLoan) => {
        const scvMap = {
            "amount": convertToScvI128(flashLoan.amount),
            "asset": convertToScvAddress(process.env[`SLENDER_TOKEN_${flashLoan.asset}`]),
            "borrow": convertToScvBool(flashLoan.borrow)
        };
        return convertToScvMap(scvMap);
    });

    return client.sendTransaction(
        process.env.SLENDER_POOL,
        "flash_loan",
        signer,
        convertToScvAddress(signer.publicKey()),
        convertToScvAddress(receiver),
        convertToScvVec(toConvert),
        convertToScvBytes("00", "hex"),
    );
}

export async function initializeFlashLoanReceiver(client: SorobanClient, signer: Keypair, receiverAddress: string): Promise<SendTransactionResult> {
    return client.sendTransaction(
        receiverAddress,
        "initialize",
        signer,
        convertToScvAddress(process.env.SLENDER_POOL),
        convertToScvBool(false),
    );
}

export function writeBudgetSnapshot(label: string, transactionResult: SendTransactionResult) {
    if (transactionResult.simulation !== null && transactionResult.simulation !== undefined) {
        const resources = transactionResult.simulation.transactionData.build().resources();
        fs.writeFileSync(BUDGET_SNAPSHOT_FILE,
            `${JSON.stringify({
                [label]: {
                    cost: transactionResult.simulation.cost,
                    events: transactionResult.simulation.events.reduce((acc, e) => acc + e.toXDR("base64").length, 0),
                    readBytes: resources.readBytes(),
                    writeBytes: resources.writeBytes(),
                    ledgerReads: resources.footprint().readOnly().length,
                    ledgerWrites: resources.footprint().readWrite().length,
                    envelopeXdr: transactionResult.response.envelopeXdr.toXDR("base64")
                }
            }, null, 2)}\n`, { flag: 'a' });
    }
}

async function initContract<T>(
    name: string,
    callback: () => Promise<SendTransactionResult>,
    success: (result: T) => string = undefined
): Promise<void> {
    name = `SLENDER_${name}`;

    if (process.env[name])
        return;

    const result = await callback();

    if (result.response.status == "SUCCESS") {
        setEnv(name, success && success(parseMetaXdrToJs(result.response.resultMetaXdr)) || "TRUE");
    } else {
        throw Error(`Transaction failed: ${name} ${JSON.stringify(result)}`);
    }
}

async function initToken(client: SorobanClient, asset: SlenderAsset, name: string): Promise<void> {
    await initContract(
        `TOKEN_${asset}_INITIALIZED`,
        () => client.sendTransaction(
            process.env[`SLENDER_TOKEN_${asset}`],
            "initialize",
            adminKeys,
            convertToScvAddress(adminKeys.publicKey()),
            convertToScvU32(9),
            convertToScvString(name),
            convertToScvString(asset)
        )
    );
}

async function initSToken(client: SorobanClient, asset: SlenderAsset, salt: string): Promise<void> {
    await initContract<Array<any>>(
        `S_TOKEN_${asset}`,
        () => client.sendTransaction(
            process.env.SLENDER_DEPLOYER,
            "deploy_s_token",
            adminKeys,
            convertToScvBytes(salt, "hex"),
            convertToScvBytes(process.env.SLENDER_S_TOKEN_HASH, "hex"),
            convertToScvString(`SToken ${asset}`),
            convertToScvString(`S${asset}`),
            convertToScvAddress(process.env.SLENDER_POOL),
            convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
        ),
        result => result[0]
    );
}

async function initDToken(client: SorobanClient, asset: SlenderAsset, salt: string): Promise<void> {
    await initContract<Array<any>>(
        `DEBT_TOKEN_${asset}`,
        () => client.sendTransaction(
            process.env.SLENDER_DEPLOYER,
            "deploy_debt_token",
            adminKeys,
            convertToScvBytes(salt, "hex"),
            convertToScvBytes(process.env.SLENDER_DEBT_TOKEN_HASH, "hex"),
            convertToScvString(`DToken ${asset}`),
            convertToScvString(`D${asset}`),
            convertToScvAddress(process.env.SLENDER_POOL),
            convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
        ),
        result => result[0]);
}

async function initPool(client: SorobanClient, salt: string): Promise<void> {
    await initContract<Array<any>>(
        "POOL",
        () => client.sendTransaction(
            process.env.SLENDER_DEPLOYER,
            "deploy_pool",
            adminKeys,
            convertToScvBytes(salt, "hex"),
            convertToScvBytes(process.env.SLENDER_POOL_HASH, "hex"),
            convertToScvAddress(adminKeys.publicKey()),
            convertToScvAddress(treasuryKeys.publicKey()),
            convertToScvU32(5),
            convertToScvMap({
                "alpha": convertToScvU32(143),
                "initial_rate": convertToScvU32(200),
                "max_rate": convertToScvU32(50_000),
                "scaling_coeff": convertToScvU32(9_000)
            })
        ),
        result => result[0]
    );
}

async function initPoolReserve(client: SorobanClient, asset: SlenderAsset, decimals: number): Promise<void> {
    await initContract(
        `POOL_${asset}_RESERVE_INITIALIZED`,
        () => client.sendTransaction(
            process.env.SLENDER_POOL,
            "init_reserve",
            adminKeys,
            convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
            convertToScvMap({
                "debt_token_address": convertToScvAddress(process.env[`SLENDER_DEBT_TOKEN_${asset}`]),
                // "decimals": convertToScvU32(9),
                "s_token_address": convertToScvAddress(process.env[`SLENDER_S_TOKEN_${asset}`]),
            })
        )
    );
}

async function initPoolCollateral(client: SorobanClient, asset: SlenderAsset): Promise<void> {
    await initContract(
        `POOL_${asset}_COLLATERAL_CONFIGURED`,
        () => client.sendTransaction(
            process.env.SLENDER_POOL,
            "configure_as_collateral",
            adminKeys,
            convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
            convertToScvMap({
                "discount": convertToScvU32(6000),
                "liq_bonus": convertToScvU32(11000),
                "liq_cap": convertToScvI128(1000000000000000n),
                "util_cap": convertToScvU32(9000)
            })
        )
    );
}

async function initPoolPriceFeed(client: SorobanClient, feed: string, assets: string[]): Promise<void> {
    await initContract(
        "POOL_PRICE_FEED_SET",
        () => client.sendTransaction(
            process.env.SLENDER_POOL,
            "set_price_feed",
            adminKeys,
            convertToScvAddress(feed),
            convertToScvVec(assets.map(asset => convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`])))
        )
    );
}

async function initPoolBorrowing(client: SorobanClient, asset: SlenderAsset): Promise<void> {
    await initContract(
        `POOL_${asset}_BORROWING_ENABLED`,
        () => client.sendTransaction(
            process.env.SLENDER_POOL,
            "enable_borrowing_on_reserve",
            adminKeys,
            convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
            convertToScvBool(true)
        )
    );
}

async function initBaseAsset(client: SorobanClient, asset: SlenderAsset): Promise<void> {
    await initContract(
        `POOL_${asset}_BASE_ASSET`,
        () => client.sendTransaction(
            process.env.SLENDER_POOL,
            "set_base_asset",
            adminKeys,
            convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
            convertToScvBool(true)
        )
    );
}
