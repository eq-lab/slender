import { Account, Keypair, SorobanRpc } from "soroban-client";
import { SorobanClient } from "./soroban.client";
import { adminKeys, setEnv, treasuryKeys } from "./soroban.config";
import { addressToScVal, arrayToScVal, boolToScVal, i128ToScVal, numberToScvU32, objectToScVal, scValStrToJs, stringToScvBytes, stringToScvString } from "./soroban.converter";

export type SlenderAsset = "XLM" | "XRP" | "USDC";

export async function init(client: SorobanClient): Promise<void> {
    let salt = 0;
    const generateSalt = (value: number): string => String(value).padStart(64, '0');

    await initToken(client, "XLM", "Lumne");
    await initToken(client, "XRP", "Ripple");
    await initToken(client, "USDC", "USD Coin");

    await initPool(client, `${generateSalt(++salt)}`);

    await initSToken(client, "XLM", `${generateSalt(++salt)}`);
    await initSToken(client, "XRP", `${generateSalt(++salt)}`);
    await initSToken(client, "USDC", `${generateSalt(++salt)}`);

    await initDToken(client, "XLM", `${generateSalt(++salt)}`);
    await initDToken(client, "XRP", `${generateSalt(++salt)}`);
    await initDToken(client, "USDC", `${generateSalt(++salt)}`);

    await initPoolReserve(client, "XLM");
    await initPoolReserve(client, "XRP");
    await initPoolReserve(client, "USDC");

    await initPoolCollateral(client, "XLM");
    await initPoolCollateral(client, "XRP");
    await initPoolCollateral(client, "USDC");

    await initPoolBorrowing(client, "XLM");
    await initPoolBorrowing(client, "XRP");
    await initPoolBorrowing(client, "USDC");

    await initPoolPriceFeed(client, process.env.SLENDER_PRICE_FEED, ["XLM", "XRP", "USDC"]);
}

export async function registerAccount(
    client: SorobanClient,
    name: string,
    keys: Keypair
): Promise<Keypair> {
    await registerAddress(
        `${name}_REGISTERED`,
        () => client.registerAccount(keys.publicKey()));

    return keys;
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
            addressToScVal(to),
            i128ToScVal(amount)
        )
    );
}

export async function balanceOf(
    client: SorobanClient,
    caller: Keypair,
    address: string,
    asset: SlenderAsset,
): Promise<bigint> {
    let result = await client.sendTransaction(
        process.env[`SLENDER_TOKEN_${asset}`],
        "balance",
        caller,
        addressToScVal(address)
    );

    return scValStrToJs(result.resultMetaXdr);
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
        setEnv(name, success && success(scValStrToJs(result.resultMetaXdr)) || "TRUE");
    } else {
        throw Error(`Transaction failed: ${name}`);
    }
}

async function registerAddress(
    name: string,
    callback: () => Promise<Account>
): Promise<void> {
    name = `SLENDER_${name}`;

    if (process.env[name])
        return;

    const result = await callback();

    if (result.accountId()) {
        setEnv(name, result.accountId());
    } else {
        throw Error(`Account registration failed: ${name}`);
    }
}

async function initToken(client: SorobanClient, asset: SlenderAsset, name: string): Promise<void> {
    await initContract(
        `TOKEN_${asset}_INITIALIZED`,
        () => client.sendTransaction(
            process.env[`SLENDER_TOKEN_${asset}`],
            "initialize",
            adminKeys,
            addressToScVal(adminKeys.publicKey()),
            numberToScvU32(9),
            stringToScvString(name),
            stringToScvString(asset)
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
            stringToScvBytes(salt, "hex"),
            stringToScvBytes(process.env.SLENDER_S_TOKEN_HASH, "hex"),
            stringToScvString(`SToken ${asset}`),
            stringToScvString(`S${asset}`),
            addressToScVal(process.env.SLENDER_POOL),
            addressToScVal(process.env[`SLENDER_TOKEN_${asset}`]),
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
            stringToScvBytes(salt, "hex"),
            stringToScvBytes(process.env.SLENDER_DEBT_TOKEN_HASH, "hex"),
            stringToScvString(`DToken ${asset}`),
            stringToScvString(`D${asset}`),
            addressToScVal(process.env.SLENDER_POOL),
            addressToScVal(process.env[`SLENDER_TOKEN_${asset}`]),
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
            stringToScvBytes(salt, "hex"),
            stringToScvBytes(process.env.SLENDER_POOL_HASH, "hex"),
            addressToScVal(adminKeys.publicKey()),
            addressToScVal(treasuryKeys.publicKey()),
            objectToScVal({
                "alpha": numberToScvU32(143),
                "initial_rate": numberToScvU32(200),
                "max_rate": numberToScvU32(50_000),
                "scaling_coeff": numberToScvU32(9_000)
            })
        ),
        result => result[0]
    );
}

async function initPoolReserve(client: SorobanClient, asset: SlenderAsset): Promise<void> {
    await initContract(
        `POOL_${asset}_RESERVE_INITIALIZED`,
        () => client.sendTransaction(
            process.env.SLENDER_POOL,
            "init_reserve",
            adminKeys,
            addressToScVal(process.env[`SLENDER_TOKEN_${asset}`]),
            objectToScVal({
                "debt_token_address": addressToScVal(process.env[`SLENDER_DEBT_TOKEN_${asset}`]),
                "s_token_address": addressToScVal(process.env[`SLENDER_S_TOKEN_${asset}`])
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
            addressToScVal(process.env[`SLENDER_TOKEN_${asset}`]),
            objectToScVal({
                "discount": numberToScvU32(6000),
                "liq_bonus": numberToScvU32(11000),
                "liq_cap": i128ToScVal(1000000000000000n),
                "util_cap": numberToScvU32(9000)
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
            addressToScVal(feed),
            arrayToScVal(assets.map(asset => addressToScVal(process.env[`SLENDER_TOKEN_${asset}`])))
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
            addressToScVal(process.env[`SLENDER_TOKEN_${asset}`]),
            boolToScVal(true)
        )
    );
}