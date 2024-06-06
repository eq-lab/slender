use pool_interface::types::error::Error;
use pool_interface::types::pause_info::PauseInfo;
use pool_interface::types::pool_config::PoolConfig;
use pool_interface::types::price_feed_config::PriceFeedConfig;
use pool_interface::types::price_feed_config_input::PriceFeedConfigInput;
use pool_interface::types::reserve_data::ReserveData;
use pool_interface::types::user_config::UserConfiguration;
use soroban_sdk::{assert_with_error, contracttype, vec, Address, Env, Vec};

pub(crate) const DAY_IN_LEDGERS: u32 = 17_280;

pub(crate) const LOW_USER_DATA_BUMP_LEDGERS: u32 = 10 * DAY_IN_LEDGERS; // 20 days
pub(crate) const HIGH_USER_DATA_BUMP_LEDGERS: u32 = 20 * DAY_IN_LEDGERS; // 30 days

pub(crate) const LOW_INSTANCE_BUMP_LEDGERS: u32 = DAY_IN_LEDGERS; // 1 day
pub(crate) const HIGH_INSTANCE_BUMP_LEDGERS: u32 = 7 * DAY_IN_LEDGERS; // 7 days

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Reserves,
    ReserveAssetKey(Address),
    UserConfig(Address),
    PriceFeed(Address),
    Pause,
    TokenSupply(Address),
    TokenBalance(Address, Address),
    PoolConfig,
    ProtocolFeeVault(Address),
}

pub fn has_admin(env: &Env) -> bool {
    bump_instance(env);

    env.storage().instance().has(&DataKey::Admin)
}

pub fn write_admin(env: &Env, admin: &Address) {
    bump_instance(env);

    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn read_admin(env: &Env) -> Result<Address, Error> {
    bump_instance(env);

    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::Uninitialized)
}

pub fn read_reserve(env: &Env, asset: &Address) -> Result<ReserveData, Error> {
    bump_instance(env);

    env.storage()
        .instance()
        .get(&DataKey::ReserveAssetKey(asset.clone()))
        .ok_or(Error::Uninitialized)
}

pub fn write_reserve(env: &Env, asset: &Address, reserve_data: &ReserveData) {
    bump_instance(env);

    let asset_key: DataKey = DataKey::ReserveAssetKey(asset.clone());
    env.storage().instance().set(&asset_key, reserve_data);
}

pub fn read_reserves(env: &Env) -> Vec<Address> {
    bump_instance(env);

    env.storage()
        .instance()
        .get(&DataKey::Reserves)
        .unwrap_or(vec![env])
}

pub fn write_reserves(env: &Env, reserves: &Vec<Address>) {
    bump_instance(env);

    env.storage().instance().set(&DataKey::Reserves, reserves);
}

pub fn read_user_config(env: &Env, user: &Address) -> Result<UserConfiguration, Error> {
    let key = DataKey::UserConfig(user.clone());
    let user_config = env.storage().persistent().get(&key);

    if user_config.is_some() {
        env.storage().persistent().extend_ttl(
            &key,
            LOW_USER_DATA_BUMP_LEDGERS,
            HIGH_USER_DATA_BUMP_LEDGERS,
        );
    }

    user_config.ok_or(Error::Uninitialized)
}

pub fn write_user_config(env: &Env, user: &Address, config: &UserConfiguration) {
    let key = DataKey::UserConfig(user.clone());
    env.storage().persistent().set(&key, config);
    env.storage().persistent().extend_ttl(
        &key,
        LOW_USER_DATA_BUMP_LEDGERS,
        HIGH_USER_DATA_BUMP_LEDGERS,
    );
}

pub fn read_price_feeds(env: &Env, asset: &Address) -> Result<PriceFeedConfig, Error> {
    bump_instance(env);

    let data_key = DataKey::PriceFeed(asset.clone());

    env.storage()
        .instance()
        .get(&data_key)
        .ok_or(Error::Uninitialized)
}

pub fn write_price_feeds(env: &Env, inputs: &Vec<PriceFeedConfigInput>) {
    bump_instance(env);

    for input in inputs.iter() {
        let data_key = DataKey::PriceFeed(input.asset.clone());

        env.storage().instance().set(
            &data_key,
            &PriceFeedConfig {
                asset_decimals: input.asset_decimals,
                min_sanity_price_in_base: input.min_sanity_price_in_base,
                max_sanity_price_in_base: input.max_sanity_price_in_base,
                feeds: input.feeds,
            },
        );
    }
}

pub fn read_pause_info(env: &Env) -> Result<PauseInfo, Error> {
    bump_instance(env);

    env.storage()
        .instance()
        .get(&DataKey::Pause)
        .ok_or(Error::Uninitialized)
}

pub fn write_pause_info(env: &Env, value: PauseInfo) {
    bump_instance(env);

    env.storage().instance().set(&DataKey::Pause, &value);
}

pub fn add_token_balance(
    env: &Env,
    token: &Address,
    account: &Address,
    amount: i128,
) -> Result<i128, Error> {
    let mut balance = read_token_balance(env, token, account);

    balance = balance
        .checked_add(amount)
        .ok_or(Error::MathOverflowError)?;

    write_token_balance(env, token, account, balance)?;

    Ok(balance)
}

pub fn read_token_total_supply(env: &Env, token: &Address) -> i128 {
    bump_instance(env);

    let key = DataKey::TokenSupply(token.clone());
    env.storage().instance().get(&key).unwrap_or(0i128)
}

pub fn write_token_total_supply(
    env: &Env,
    token: &Address,
    total_supply: i128,
) -> Result<(), Error> {
    bump_instance(env);

    assert_with_error!(env, !total_supply.is_negative(), Error::MustBeNonNegative);

    let data_key = DataKey::TokenSupply(token.clone());
    env.storage().instance().set(&data_key, &total_supply);

    Ok(())
}

pub fn read_token_balance(env: &Env, token: &Address, account: &Address) -> i128 {
    let key = DataKey::TokenBalance(token.clone(), account.clone());
    let balance = env.storage().persistent().get(&key);

    if balance.is_some() {
        env.storage().persistent().extend_ttl(
            &key,
            LOW_USER_DATA_BUMP_LEDGERS,
            HIGH_USER_DATA_BUMP_LEDGERS,
        );
    }

    balance.unwrap_or(0i128)
}

pub fn write_token_balance(
    env: &Env,
    token: &Address,
    account: &Address,
    balance: i128,
) -> Result<(), Error> {
    assert_with_error!(env, !balance.is_negative(), Error::MustBeNonNegative);

    let key = DataKey::TokenBalance(token.clone(), account.clone());
    env.storage().persistent().set(&key, &balance);
    env.storage().persistent().extend_ttl(
        &key,
        LOW_USER_DATA_BUMP_LEDGERS,
        HIGH_USER_DATA_BUMP_LEDGERS,
    );

    Ok(())
}

pub fn read_protocol_fee_vault(env: &Env, asset: &Address) -> i128 {
    bump_instance(env);

    let key = DataKey::ProtocolFeeVault(asset.clone());
    let value = env.storage().instance().get(&key);

    value.unwrap_or(0)
}

pub fn write_protocol_fee_vault(env: &Env, asset: &Address, balance: i128) {
    assert_with_error!(env, !balance.is_negative(), Error::MustBeNonNegative);
    let key = DataKey::ProtocolFeeVault(asset.clone());

    env.storage().instance().set(&key, &balance);
    bump_instance(env);
}

pub fn add_protocol_fee_vault(env: &Env, asset: &Address, amount: i128) -> Result<(), Error> {
    let mut balance = read_protocol_fee_vault(env, asset);
    balance = balance
        .checked_add(amount)
        .ok_or(Error::MathOverflowError)?;

    write_protocol_fee_vault(env, asset, balance);

    Ok(())
}

pub fn write_pool_config(env: &Env, config: &PoolConfig) {
    bump_instance(env);

    env.storage()
        .instance()
        .set(&DataKey::PoolConfig, &config.clone());
}

pub fn read_pool_config(env: &Env) -> Result<PoolConfig, Error> {
    bump_instance(env);

    env.storage()
        .instance()
        .get(&DataKey::PoolConfig)
        .ok_or(Error::Uninitialized)
}

fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);
}
