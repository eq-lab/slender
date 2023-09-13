import { Address, Keypair, SorobanRpc } from "soroban-client";
import { SorobanClient } from "./soroban.client";
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

export type SlenderAsset = "XLM" | "XRP" | "USDC";

export interface MintBurn {
    asset_balance: Map<string, any>;
    mint: boolean;
    who: Address;
}

export interface AccountPosition {
    debt: bigint;
    discounted_collateral: bigint;
    npv: bigint;
}

export async function init(client: SorobanClient): Promise<void> {
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

    await initPoolPriceFeed(client, process.env.SLENDER_PRICE_FEED, ["XLM", "XRP", "USDC"]);
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

export async function mintBurn(
    client: SorobanClient,
    mintsBurns: Array<MintBurn>
): Promise<void> {
    for (let i = 0; i < mintsBurns.length; i++) {
        const response = await client.sendTransaction(
            mintsBurns[i].asset_balance.get("asset"),
            mintsBurns[i].mint ? "mint" : "clawback",
            adminKeys,
            convertToScvAddress(mintsBurns[i].who.toString()),
            convertToScvI128(mintsBurns[i].asset_balance.get("balance"))
        );

        if (response.status != "SUCCESS") {
            throw Error("Failed to transfer tokens!");
        }
    }
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
    amount: bigint
): Promise<void> {
    await client.sendTransaction(
        process.env.SLENDER_POOL,
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
    amount: bigint
): Promise<void> {
    const response = await client.sendTransaction(
        process.env.SLENDER_POOL,
        "borrow",
        signer,
        convertToScvAddress(signer.publicKey()),
        convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
        convertToScvI128(amount)
    );

    const result = parseMetaXdrToJs<Array<MintBurn>>(
        response.resultMetaXdr
    );

    await mintBurn(client, result);
}

export async function deposit(
    client: SorobanClient,
    signer: Keypair,
    asset: SlenderAsset,
    amount: bigint
): Promise<void> {
    const response = await client.sendTransaction(
        process.env.SLENDER_POOL,
        "deposit",
        signer,
        convertToScvAddress(signer.publicKey()),
        convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
        convertToScvI128(amount)
    );

    const result = parseMetaXdrToJs<Array<MintBurn>>(
        response.resultMetaXdr
    );

    await mintBurn(client, result);
}

export async function repay(
    client: SorobanClient,
    signer: Keypair,
    asset: SlenderAsset,
    amount: bigint
): Promise<void> {
    const response = await client.sendTransaction(
        process.env.SLENDER_POOL,
        "repay",
        signer,
        convertToScvAddress(signer.publicKey()),
        convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
        convertToScvI128(amount)
    );

    const result = parseMetaXdrToJs<Array<MintBurn>>(
        response.resultMetaXdr
    );

    await mintBurn(client, result);
}

export async function withdraw(
    client: SorobanClient,
    signer: Keypair,
    asset: SlenderAsset,
    amount: bigint
): Promise<void> {
    const response = await client.sendTransaction(
        process.env.SLENDER_POOL,
        "withdraw",
        signer,
        convertToScvAddress(signer.publicKey()),
        convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
        convertToScvI128(amount),
        convertToScvAddress(signer.publicKey())
    );

    const result = parseMetaXdrToJs<Array<MintBurn>>(
        response.resultMetaXdr
    );

    await mintBurn(client, result);
}

export async function liquidate(
    client: SorobanClient,
    signer: Keypair,
    who: string,
    receiveStoken: boolean
): Promise<void> {
    const response = await client.sendTransaction(
        process.env.SLENDER_POOL,
        "liquidate",
        signer,
        convertToScvAddress(signer.publicKey()),
        convertToScvAddress(who),
        convertToScvBool(receiveStoken)
    );

    const result = parseMetaXdrToJs<Array<MintBurn>>(
        response.resultMetaXdr
    );

    await mintBurn(client, result);
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

export async function deploy(): Promise<void> {
    await new Promise((resolve, reject) => {
        exec(`../deploy/scripts/deploy.sh ${process.env.NODE_ENV}`, (error, stdout, _) => {
            if (error) {
                reject(error);
                return;
            }
            require("dotenv").config({ path: contractsFilename });

            resolve(stdout)
        });
    });
}

export async function cleanSlenderEnvKeys() {
    Object.keys(process.env).forEach(key => {
        if (key.startsWith("SLENDER_")) {
            delete process.env[key];
        }
    });
}

async function initContract<T>(
    name: string,
    callback: () => Promise<SorobanRpc.GetTransactionResponse>,
    success: (result: T) => string = undefined
): Promise<void> {
    name = `SLENDER_${name}`;

    if (process.env[name])
        return;

    const result = await callback();

    if (result.status == "SUCCESS") {
        setEnv(name, success && success(parseMetaXdrToJs(result.resultMetaXdr)) || "TRUE");
    } else {
        throw Error(`Transaction failed: ${name}`);
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
                // "decimals": convertToScvU32(decimals),
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
