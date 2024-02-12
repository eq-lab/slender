import { Keypair, xdr } from "stellar-sdk";
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
    parseScvToJs,
    convertToScvU64,
    convertToScvEnum,
} from "./soroban.converter";
import { exec } from "child_process";
import * as fs from "fs";

export const I128_MAX = 170_141_183_460_469_231_731_687_303_715_884_105_727n;

export const BUDGET_SNAPSHOT_FILE = "snapshots/budget_utilization.snap";

export type SlenderAsset = "XLM" | "XRP" | "USDC" | "RWA";

export interface AccountPosition {
    debt: bigint;
    discounted_collateral: bigint;
    npv: bigint;
}

export interface FlashLoanAsset {
    asset: SlenderAsset;
    amount: bigint;
    borrow: boolean;
}

interface PriceFeedConfig {
    feed: string;
    feed_asset: SlenderAsset;
    feed_decimals: number;
    twap_records: number;
}

interface PriceData {
    price: bigint;
    timestamp: bigint;
}

export function healthFactor(accountPosition: AccountPosition): number {
    return Number(accountPosition.npv) / Number(accountPosition.discounted_collateral);
}

export async function init(client: SorobanClient, customXlm = true): Promise<void> {
    console.log("    Contracts initialization has been started");

    require("dotenv").config({ path: contractsFilename });

    let salt = 0;
    const generateSalt = (value: number): string =>
        String(value).padStart(64, "0");

    if (customXlm) {
        await initToken(client, "XLM", "Lumens", 7);
    }

    await initToken(client, "XRP", "Ripple", 9);
    await initToken(client, "USDC", "USD Coin", 9);
    await initToken(client, "RWA", "RWA asset", 9);

    await initPool(client, `${generateSalt(++salt)}`);
    // need to create treasury account to be able to receive native XLM token
    await client.registerAccount(treasuryKeys.publicKey());

    await initSToken(client, "XRP", `${generateSalt(++salt)}`);
    await initSToken(client, "USDC", `${generateSalt(++salt)}`);
    await initSToken(client, "XLM", `${generateSalt(++salt)}`);

    await initDToken(client, "XLM", `${generateSalt(++salt)}`);
    await initDToken(client, "XRP", `${generateSalt(++salt)}`);
    await initDToken(client, "USDC", `${generateSalt(++salt)}`);

    await initPoolReserve(client, "XLM");
    await initPoolReserve(client, "XRP");
    await initPoolReserve(client, "USDC");
    await initPoolReserve(client, "RWA", false);

    await initPoolCollateral(client, "XRP", 1);
    await initPoolCollateral(client, "USDC", 2);
    await initPoolCollateral(client, "XLM", 3);
    await initPoolCollateral(client, "RWA", 4);

    await initPoolBorrowing(client, "XLM");
    await initPoolBorrowing(client, "XRP");
    await initPoolBorrowing(client, "USDC");

    await initBaseAsset(client, "XLM", 7);

    await initPrice(client, "XLM", 100_000_000_000_000n, 0);
    await initPrice(client, "XRP", 10_000_000_000_000_000n, 0);
    await initPrice(client, "USDC", 10_000_000_000_000_000n, 0);
    await initPrice(client, "RWA", 10_000_000_000_000_000n, 0);

    await initPoolPriceFeed(client, [
        {
        asset: "XLM",
        asset_decimals: 7,
        priceFeedConfig: {
            feed_asset: "XLM",
            feed_decimals: 14,
            feed: process.env.SLENDER_PRICE_FEED,
                twap_records: 1,
            },
        },
        {
        asset: "XRP",
        asset_decimals: 9,
        priceFeedConfig: {
            feed_asset: "XRP",
            feed_decimals: 16,
            feed: process.env.SLENDER_PRICE_FEED,
                twap_records: 1,
            },
        },
        {
        asset: "USDC",
        asset_decimals: 9,
        priceFeedConfig: {
            feed_asset: "USDC",
            feed_decimals: 16,
            feed: process.env.SLENDER_PRICE_FEED,
                twap_records: 1,
            },
        },
        {
            asset: "RWA",
            asset_decimals: 9,
            priceFeedConfig: {
                feed_asset: "RWA",
                feed_decimals: 16,
                feed: process.env.SLENDER_PRICE_FEED,
                twap_records: 1,
            },
        },
    ]);

    console.log("    Contracts initialization has been finished");
}

export async function mintUnderlyingTo(
    client: SorobanClient,
    asset: SlenderAsset,
    to: string,
    amount: bigint
): Promise<void> {
    await initContract(`${to}_${asset}_MINTED`, () =>
        client.sendTransaction(
            process.env[`SLENDER_TOKEN_${asset}`],
            "mint",
            adminKeys,
            3,
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
    signer: Keypair
): Promise<AccountPosition> {
    const xdrResponse = await client.simulateTransaction(
        process.env.SLENDER_POOL,
        "account_position",
        convertToScvAddress(signer.publicKey())
    );

    return parseScvToJs<AccountPosition>(xdrResponse);
}

export async function initPriceFeed(
    client: SorobanClient,
    asset: SlenderAsset,
    amount: bigint,
    decimals: number
): Promise<SendTransactionResult> {
    return client.sendTransaction(
        process.env.SLENDER_PRICE_FEED,
        "init",
        adminKeys,
        3,
        convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
        convertToScvI128(amount),
        convertToScvU32(decimals)
    );
}

export async function sTokenUnderlyingBalanceOf(
    client: SorobanClient,
    asset: SlenderAsset
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
    asset: SlenderAsset
): Promise<bigint> {
    const xdrResponse = await client.simulateTransaction(
        process.env[`SLENDER_S_TOKEN_${asset}`],
        "total_supply"
    );

    return parseScvToJs(xdrResponse);
}

export async function debtTokenTotalSupply(
    client: SorobanClient,
    asset: SlenderAsset
): Promise<bigint> {
    const xdrResponse = await client.simulateTransaction(
        process.env[`SLENDER_DEBT_TOKEN_${asset}`],
        "total_supply"
    );

    return parseScvToJs(xdrResponse);
}

export async function borrow(
    client: SorobanClient,
    signer: Keypair,
    asset: SlenderAsset,
    amount: bigint
): Promise<SendTransactionResult> {
    const txResult = await client.sendTransaction(
        process.env.SLENDER_POOL,
        "borrow",
        signer,
        3,
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
    amount: bigint
): Promise<SendTransactionResult> {
    const txResult = await client.sendTransaction(
        process.env.SLENDER_POOL,
        "deposit",
        signer,
        3,
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
    amount: bigint
): Promise<SendTransactionResult> {
    const txResult = await client.sendTransaction(
        process.env.SLENDER_POOL,
        "repay",
        signer,
        3,
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
    amount: bigint
): Promise<SendTransactionResult> {
    const txResult = await client.sendTransaction(
        process.env.SLENDER_POOL,
        "withdraw",
        signer,
        3,
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
    receiveStoken: boolean
): Promise<SendTransactionResult> {
    const txResult = await client.sendTransaction(
        process.env.SLENDER_POOL,
        "liquidate",
        signer,
        10,
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
    amount: bigint
): Promise<SendTransactionResult> {
    return client.sendTransaction(
        process.env[`SLENDER_S_TOKEN_${asset}`],
        "transfer",
        signer,
        3,
        convertToScvAddress(signer.publicKey()),
        convertToScvAddress(to),
        convertToScvI128(amount)
    );
}

export async function deploy(): Promise<void> {
    console.log("    Contracts deployment has been started");

    const stdout = await new Promise((resolve, reject) => {
        exec(
            `../deploy/scripts/deploy.sh ${process.env.NODE_ENV}`,
            (error, stdout, _) => {
                if (error) {
                    reject(error);
                    return;
                }
                resolve(stdout);
            }
        );
    });
    console.log(stdout);
    console.log("    Contracts deployment has been finished");
}

export async function deployReceiverMock(): Promise<string> {
    console.log("    Flashloan receiver deployment has been started");

    const flashLoadReceiverMockAddress = (
        (await new Promise((resolve, reject) => {
            exec(
        `soroban contract deploy \
        --wasm ../target/wasm32-unknown-unknown/release/flash_loan_receiver_mock.wasm \
        --source ${adminKeys.secret()} \
        --rpc-url "${process.env.SOROBAN_RPC_URL}" \
        --network-passphrase "${process.env.PASSPHRASE}"`,
                (error, stdout, _) => {
                    if (error) {
                        reject(error);
                        return;
                    }
                    resolve(stdout);
                }
            );
        })) as string
    ).trim();

    setEnv("SLENDER_FLASHLOAN_RECEIVER_MOCK", flashLoadReceiverMockAddress);

    console.log("    Flashloan receiver deployment has been finished");

    return (flashLoadReceiverMockAddress as string).trim();
}

export async function liquidateCli(
    liquidatorKeys: Keypair,
    borrower: string,
    debtAsset: SlenderAsset,
    receiveStoken: boolean
): Promise<string> {
    const liquidateResult = (
        (await new Promise((resolve) => {
            exec(
        `soroban --very-verbose contract invoke \
        --id ${process.env.SLENDER_POOL} \
        --source ${liquidatorKeys.secret()} \
        --rpc-url "${process.env.SOROBAN_RPC_URL}" \
        --network-passphrase "${process.env.PASSPHRASE}" \
        -- \
        liquidate \
        --liquidator ${liquidatorKeys.publicKey()} \
        --who ${borrower} \
        --debt_asset ${process.env[`SLENDER_TOKEN_${debtAsset}`]} \
        --receive_stoken ${receiveStoken}`,
                (error, stdout, stderr) => {
                    if (error) {
                        resolve(stderr);
                        return;
                    }
                    resolve(stdout);
                }
            );
        })) as string
    ).trim();

    return liquidateResult;
}

export async function cleanSlenderEnvKeys() {
    Object.keys(process.env).forEach((key) => {
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
        3,
        convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
        convertToScvAddress(signer.publicKey()),
        convertToScvAddress(to),
        convertToScvI128(amount),
        convertToScvI128(from_before),
        convertToScvI128(to_before),
        convertToScvI128(total_supply)
    );
}

export async function flashLoan(
    client: SorobanClient,
    signer: Keypair,
    receiver: string,
    loanAssets: FlashLoanAsset[],
    params: string
): Promise<SendTransactionResult> {
    const toConvert = loanAssets.map((flashLoan) =>
        convertToScvMap({
            amount: convertToScvI128(flashLoan.amount),
            asset: convertToScvAddress(
                process.env[`SLENDER_TOKEN_${flashLoan.asset}`]
            ),
            borrow: convertToScvBool(flashLoan.borrow),
        })
    );

    return client.sendTransaction(
        process.env.SLENDER_POOL,
        "flash_loan",
        signer,
        3,
        convertToScvAddress(signer.publicKey()),
        convertToScvAddress(receiver),
        convertToScvVec(toConvert),
        convertToScvBytes("00", "hex")
    );
}

export async function initializeFlashLoanReceiver(
    client: SorobanClient,
    signer: Keypair,
    receiverAddress: string,
    shouldFail: boolean
): Promise<SendTransactionResult> {
    return client.sendTransaction(
        receiverAddress,
        "initialize",
        signer,
        3,
        convertToScvAddress(process.env.SLENDER_POOL),
        convertToScvBool(shouldFail)
    );
}

export function writeBudgetSnapshot(
    label: string,
    transactionResult: SendTransactionResult
) {
    if (
        transactionResult.simulation !== null &&
        transactionResult.simulation !== undefined
    ) {
        const resources = transactionResult.simulation.transactionData
            .build()
            .resources();
        fs.writeFileSync(
            BUDGET_SNAPSHOT_FILE,
            `${JSON.stringify(
                {
                    [label]: {
                        cost: transactionResult.simulation.cost,
                        events: transactionResult.simulation.events.reduce(
                            (acc, e) => acc + e.event().toXDR().length,
                            0
                        ),
                        readBytes: resources.readBytes(),
                        writeBytes: resources.writeBytes(),
                        ledgerReads: resources.footprint().readOnly().length,
                        ledgerWrites: resources.footprint().readWrite().length,
                    },
                },
                null,
                2
            )}\n`,
            { flag: "a" }
        );
    }
}

export async function readPriceFeed(
    client: SorobanClient,
    asset: SlenderAsset
): Promise<PriceFeedConfig> {
    const xdrResponse = await client.simulateTransaction(
        process.env.SLENDER_POOL,
        "price_feed",
        convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`])
    );

    return parseScvToJs<PriceFeedConfig>(xdrResponse);
}

export async function readPrice(
    client: SorobanClient,
    feed: string,
    asset: SlenderAsset
): Promise<bigint> {
    const xdrResponse = await client.simulateTransaction(
        feed,
        "lastprice",
        convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`])
    );

    return parseScvToJs<PriceData>(xdrResponse).price;
}

async function initContract<T>(
    name: string,
    callback: () => Promise<SendTransactionResult>,
    success: (result: T) => string = undefined
): Promise<void> {
    name = `SLENDER_${name}`;

    if (process.env[name]) return;

    const result = await callback();

    if (result.response.status == "SUCCESS") {
        setEnv(
            name,
            (success && success(parseMetaXdrToJs(result.response.resultMetaXdr))) ||
            "TRUE"
        );
    } else {
        throw Error(`Transaction failed: ${name} ${JSON.stringify(result)}`);
    }
}

async function initToken(
    client: SorobanClient,
    asset: SlenderAsset,
    name: string,
    decimals: number
): Promise<void> {
    await initContract(`TOKEN_${asset}_INITIALIZED`, () =>
        client.sendTransaction(
            process.env[`SLENDER_TOKEN_${asset}`],
            "initialize",
            adminKeys,
            3,
            convertToScvAddress(adminKeys.publicKey()),
            convertToScvU32(decimals),
            convertToScvString(name),
            convertToScvString(asset)
        )
    );
}

async function initSToken(
    client: SorobanClient,
    asset: SlenderAsset,
    salt: string
): Promise<void> {
    await initContract<Array<any>>(
        `S_TOKEN_${asset}`,
        () =>
            client.sendTransaction(
                process.env.SLENDER_DEPLOYER,
                "deploy_s_token",
                adminKeys,
                3,
                convertToScvBytes(salt, "hex"),
                convertToScvBytes(process.env.SLENDER_S_TOKEN_HASH, "hex"),
                convertToScvString(`SToken ${asset}`),
                convertToScvString(`S${asset}`),
                convertToScvAddress(process.env.SLENDER_POOL),
                convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`])
            ),
        (result) => result[0]
    );
}

async function initDToken(
    client: SorobanClient,
    asset: SlenderAsset,
    salt: string
): Promise<void> {
    await initContract<Array<any>>(
        `DEBT_TOKEN_${asset}`,
        () =>
            client.sendTransaction(
                process.env.SLENDER_DEPLOYER,
                "deploy_debt_token",
                adminKeys,
                3,
                convertToScvBytes(salt, "hex"),
                convertToScvBytes(process.env.SLENDER_DEBT_TOKEN_HASH, "hex"),
                convertToScvString(`DToken ${asset}`),
                convertToScvString(`D${asset}`),
                convertToScvAddress(process.env.SLENDER_POOL),
                convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`])
            ),
        (result) => result[0]
    );
}

async function initPool(client: SorobanClient, salt: string): Promise<void> {
    await initContract<Array<any>>(
        "POOL",
        () =>
            client.sendTransaction(
                process.env.SLENDER_DEPLOYER,
                "deploy_pool",
                adminKeys,
                3,
                convertToScvBytes(salt, "hex"),
                convertToScvBytes(process.env.SLENDER_POOL_HASH, "hex"),
                convertToScvAddress(adminKeys.publicKey()),
                convertToScvAddress(treasuryKeys.publicKey()),
                convertToScvU32(5),
                convertToScvU32(2_500),
                convertToScvMap({
                    alpha: convertToScvU32(143),
                    initial_rate: convertToScvU32(200),
                    max_rate: convertToScvU32(50_000),
                    scaling_coeff: convertToScvU32(9_000),
                })
            ),
        (result) => result[0]
    );
}

async function initPoolReserve(
    client: SorobanClient,
    asset: SlenderAsset,
    fungible = true
): Promise<void> {
    await initContract(`POOL_${asset}_RESERVE_INITIALIZED`, () =>
        client.sendTransaction(
            process.env.SLENDER_POOL,
            "init_reserve",
            adminKeys,
            3,
            convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
            fungible ? convertToScvEnum("Fungible", [
                convertToScvAddress(process.env[`SLENDER_S_TOKEN_${asset}`]),
                convertToScvAddress(process.env[`SLENDER_DEBT_TOKEN_${asset}`]),
            ]) : convertToScvEnum("RWA")
        )
    );
}

async function initPoolCollateral(
    client: SorobanClient,
    asset: SlenderAsset,
    order: number
): Promise<void> {
    await initContract(`POOL_${asset}_COLLATERAL_CONFIGURED`, () =>
        client.sendTransaction(
            process.env.SLENDER_POOL,
            "configure_as_collateral",
            adminKeys,
            3,
            convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
            convertToScvMap({
                // todo: trim to short string
                discount: convertToScvU32(6000),
                liq_cap: convertToScvI128(1000000000000000n),
                pen_order: convertToScvU32(order),
                util_cap: convertToScvU32(9000),
            })
        )
    );
}

async function initPoolPriceFeed(
    client: SorobanClient,
    inputs: {
        asset: SlenderAsset,
        asset_decimals: number,
        priceFeedConfig: PriceFeedConfig
    }[]
): Promise<void> {
    await initContract(
        "POOL_PRICE_FEED_SET",
        () => client.sendTransaction(
            process.env.SLENDER_POOL,
            "set_price_feeds",
            adminKeys,
            3,
            convertToScvVec(inputs.map(input => convertToScvMap({
                "asset": convertToScvAddress(process.env[`SLENDER_TOKEN_${input.asset}`]),
                "asset_decimals": convertToScvU32(input.asset_decimals),
                "feeds": convertToScvVec([
                    convertToScvMap({
                        "feed": convertToScvAddress(input.priceFeedConfig.feed),
                        "feed_asset": convertToScvVec([
                            xdr.ScVal.scvSymbol("Stellar"),
                            convertToScvAddress(process.env[`SLENDER_TOKEN_${input.priceFeedConfig.feed_asset}`])
                        ]),
                        "feed_decimals": convertToScvU32(input.priceFeedConfig.feed_decimals),
                        "twap_records": convertToScvU32(input.priceFeedConfig.twap_records)
                    })
                ]),
            })))
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
            3,
            convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
            convertToScvBool(true)
        )
    );
}

async function initBaseAsset(
    client: SorobanClient,
    asset: SlenderAsset,
    decimals: number
): Promise<void> {
    await initContract(
        `POOL_${asset}_BASE_ASSET_SET`,
        () => client.sendTransaction(
            process.env.SLENDER_POOL,
            "set_base_asset",
            adminKeys,
            3,
            convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
            convertToScvU32(decimals)
        )
    );
}

export async function initPrice(
    client: SorobanClient,
    asset: SlenderAsset,
    price: bigint,
    timestamp: number,
): Promise<void> {
    await client.sendTransaction(
        process.env.SLENDER_PRICE_FEED,
        "init",
        adminKeys,
        3,
        convertToScvVec([
            xdr.ScVal.scvSymbol("Stellar"),
            convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`])
        ]),
        convertToScvVec([
            convertToScvMap({
                "price": convertToScvI128(price),
                "timestamp": convertToScvU64(timestamp)
            })
        ]),
    );
}

export async function inPoolBalanceOf(
    client: SorobanClient,
    asset: SlenderAsset,
    who: string,
): Promise<bigint> {
    const xdrResponse = await client.simulateTransaction(
        process.env.SLENDER_POOL,
        "balance",
        convertToScvAddress(who),
        convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`])
    );

    return parseScvToJs(xdrResponse);
}