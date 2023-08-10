import { Account, Keypair, SorobanRpc } from "soroban-client";
import { SorobanClient } from "./soroban.client";
import { adminKeys, borrower1Keys, setEnv, treasuryKeys } from "./soroban.config";
import { addressToScVal, arrayToScVal, boolToScVal, i128ToScVal, numberToScvU32, objectToScVal, parseScVal, stringToScvBytes, stringToScvString } from "./soroban.converter";

export async function initPool(client: SorobanClient): Promise<void> {
    // await registerAddress(
    //     "BORROWER_1",
    //     () => client.registerAccount(borrower1Keys.publicKey()));

    await initContract<Array<any>>(
        "POOL",
        () => client.sendTransaction(
            process.env.SLENDER_DEPLOYER,
            "deploy_pool",
            adminKeys,
            stringToScvBytes("0000000000000000000000000000000000000000000000000000000000000000", "hex"),
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

    await initContract(
        "TOKEN_INITIALIZED",
        () => client.sendTransaction(
            process.env.SLENDER_TOKEN,
            "initialize",
            adminKeys,
            addressToScVal(adminKeys.publicKey()),
            numberToScvU32(9),
            stringToScvString("Token"),
            stringToScvString("TKN")
        )
    );

    await initContract<Array<any>>(
        "S_TOKEN",
        () => client.sendTransaction(
            process.env.SLENDER_DEPLOYER,
            "deploy_s_token",
            adminKeys,
            stringToScvBytes("0000000000000000000000000000000000000000000000000000000000000001", "hex"),
            stringToScvBytes(process.env.SLENDER_S_TOKEN_HASH, "hex"),
            stringToScvString("SToken"),
            stringToScvString("STKN"),
            addressToScVal(process.env.SLENDER_POOL),
            addressToScVal(process.env.SLENDER_TOKEN),
        ),
        result => result[0]
    );

    await initContract<Array<any>>(
        "DEBT_TOKEN",
        () => client.sendTransaction(
            process.env.SLENDER_DEPLOYER,
            "deploy_debt_token",
            adminKeys,
            stringToScvBytes("0000000000000000000000000000000000000000000000000000000000000002", "hex"),
            stringToScvBytes(process.env.SLENDER_DEBT_TOKEN_HASH, "hex"),
            stringToScvString("DToken"),
            stringToScvString("DTKN"),
            addressToScVal(process.env.SLENDER_POOL),
            addressToScVal(process.env.SLENDER_TOKEN),
        ),
        result => result[0]);

    await initContract(
        "RESERVE_INITIALIZED",
        () => client.sendTransaction(
            process.env.SLENDER_POOL,
            "init_reserve",
            adminKeys,
            addressToScVal(process.env.SLENDER_TOKEN),
            objectToScVal({
                "debt_token_address": addressToScVal(process.env.SLENDER_DEBT_TOKEN),
                "s_token_address": addressToScVal(process.env.SLENDER_S_TOKEN)
            })
        )
    );

    await initContract(
        "COLLATERAL_CONFIGURED",
        () => client.sendTransaction(
            process.env.SLENDER_POOL,
            "configure_as_collateral",
            adminKeys,
            addressToScVal(process.env.SLENDER_TOKEN),
            objectToScVal({
                "discount": numberToScvU32(6000),
                "liq_bonus": numberToScvU32(11000),
                "liq_cap": i128ToScVal(1000000000000000n),
                "util_cap": numberToScvU32(9000)
            })
        )
    );

    await initContract(
        "PRICE_FEED_SET",
        () => client.sendTransaction(
            process.env.SLENDER_POOL,
            "set_price_feed",
            adminKeys,
            addressToScVal(process.env.SLENDER_PRICE_FEED),
            arrayToScVal([
                addressToScVal(process.env.SLENDER_TOKEN)
            ])
        )
    );

    await initContract(
        "BORROWING_ENABLED",
        () => client.sendTransaction(
            process.env.SLENDER_POOL,
            "enable_borrowing_on_reserve",
            adminKeys,
            addressToScVal(process.env.SLENDER_TOKEN),
            boolToScVal(true)
        )
    );

    await initContract(
        "BORROWER_UNDERLYING_MINTED",
        () => client.sendTransaction(
            process.env.SLENDER_TOKEN,
            "mint",
            adminKeys,
            addressToScVal(borrower1Keys.publicKey()),
            i128ToScVal(100_000_000_000_000n)
        )
    );
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
        setEnv(name, success && success(parseScVal(result.resultMetaXdr)) || "TRUE");
    } else {
        throw Error(`Transaction failed: ${name}`);
    }
}

async function registerAddress(
    name: string,
    callback: () => Promise<Account>
): Promise<void> {
    name = `SLENDER_ACC_${name}`;

    if (process.env[name])
        return;

    const result = await callback();

    if (result.accountId) {
        setEnv(name, result.accountId.toString());
    } else {
        throw Error(`Account registration failed: ${name}`);
    }
}