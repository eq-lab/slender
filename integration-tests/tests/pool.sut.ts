import { Account, Keypair, SorobanRpc } from "soroban-client";
import { SorobanClient } from "./soroban.client";
import { adminKeys, setEnv, treasuryKeys } from "./soroban.config";
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
} from "./soroban.converter";
import { expect } from "chai";

export type SlenderAsset = "XLM" | "XRP" | "USDC";

export async function init(client: SorobanClient): Promise<void> {
  let salt = 0;
  const generateSalt = (value: number): string =>
    String(value).padStart(64, "0");

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

  await initPoolPriceFeed(client, process.env.SLENDER_PRICE_FEED, [
    "XLM",
    "XRP",
    "USDC",
  ]);
}

export async function registerAccount(
  client: SorobanClient,
  name: string,
  keys: Keypair
): Promise<Keypair> {
  await registerAddress(`${name}_REGISTERED`, () =>
    client.registerAccount(keys.publicKey())
  );

  return keys;
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
      convertToScvAddress(to),
      convertToScvI128(amount)
    )
  );
}

export async function balanceOf(
  client: SorobanClient,
  caller: Keypair,
  address: string,
  asset: SlenderAsset
): Promise<bigint> {
  let result = await client.sendTransaction(
    process.env[`SLENDER_TOKEN_${asset}`],
    "balance",
    caller,
    convertToScvAddress(address)
  );

  return parseMetaXdrToJs(result.resultMetaXdr);
}

export async function deposit(
  client: SorobanClient,
  caller: Keypair,
  user: string,
  asset: SlenderAsset,
  amount: bigint
): Promise<void> {
  let result = await client.sendTransaction(
    process.env.SLENDER_POOL,
    "deposit",
    caller,
    convertToScvAddress(user),
    convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
    convertToScvI128(amount)
  );

  expect(result.status).to.equal("SUCCESS");
}

async function initContract<T>(
  name: string,
  callback: () => Promise<SorobanRpc.GetTransactionResponse>,
  success: (result: T) => string = undefined
): Promise<void> {
  name = `SLENDER_${name}`;

  if (process.env[name]) return;

  const result = await callback();

  if (result.status == "SUCCESS") {
    setEnv(
      name,
      (success && success(parseMetaXdrToJs(result.resultMetaXdr))) || "TRUE"
    );
  } else {
    throw Error(`Transaction failed: ${name}`);
  }
}

async function registerAddress(
  name: string,
  callback: () => Promise<Account>
): Promise<void> {
  name = `SLENDER_${name}`;

  if (process.env[name]) return;

  const result = await callback();

  if (result.accountId()) {
    setEnv(name, result.accountId());
  } else {
    throw Error(`Account registration failed: ${name}`);
  }
}

async function initToken(
  client: SorobanClient,
  asset: SlenderAsset,
  name: string
): Promise<void> {
  await initContract(`TOKEN_${asset}_INITIALIZED`, () =>
    client.sendTransaction(
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
        convertToScvBytes(salt, "hex"),
        convertToScvBytes(process.env.SLENDER_POOL_HASH, "hex"),
        convertToScvAddress(adminKeys.publicKey()),
        convertToScvAddress(treasuryKeys.publicKey()),
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
  asset: SlenderAsset
): Promise<void> {
  await initContract(`POOL_${asset}_RESERVE_INITIALIZED`, () =>
    client.sendTransaction(
      process.env.SLENDER_POOL,
      "init_reserve",
      adminKeys,
      convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
      convertToScvMap({
        debt_token_address: convertToScvAddress(
          process.env[`SLENDER_DEBT_TOKEN_${asset}`]
        ),
        s_token_address: convertToScvAddress(
          process.env[`SLENDER_S_TOKEN_${asset}`]
        ),
      })
    )
  );
}

async function initPoolCollateral(
  client: SorobanClient,
  asset: SlenderAsset
): Promise<void> {
  await initContract(`POOL_${asset}_COLLATERAL_CONFIGURED`, () =>
    client.sendTransaction(
      process.env.SLENDER_POOL,
      "configure_as_collateral",
      adminKeys,
      convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
      convertToScvMap({
        discount: convertToScvU32(6000),
        liq_bonus: convertToScvU32(11000),
        liq_cap: convertToScvI128(1000000000000000n),
        util_cap: convertToScvU32(9000),
      })
    )
  );
}

async function initPoolPriceFeed(
  client: SorobanClient,
  feed: string,
  assets: string[]
): Promise<void> {
  await initContract("POOL_PRICE_FEED_SET", () =>
    client.sendTransaction(
      process.env.SLENDER_POOL,
      "set_price_feed",
      adminKeys,
      convertToScvAddress(feed),
      convertToScvVec(
        assets.map((asset) =>
          convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`])
        )
      )
    )
  );
}

async function initPoolBorrowing(
  client: SorobanClient,
  asset: SlenderAsset
): Promise<void> {
  await initContract(`POOL_${asset}_BORROWING_ENABLED`, () =>
    client.sendTransaction(
      process.env.SLENDER_POOL,
      "enable_borrowing_on_reserve",
      adminKeys,
      convertToScvAddress(process.env[`SLENDER_TOKEN_${asset}`]),
      convertToScvBool(true)
    )
  );
}
