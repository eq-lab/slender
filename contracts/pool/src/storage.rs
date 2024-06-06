use pool_interface::types::error::Error;
use pool_interface::types::ir_params::IRParams;
use pool_interface::types::permission::Permission;
use pool_interface::types::price_feed_config::PriceFeedConfig;
use pool_interface::types::price_feed_config_input::PriceFeedConfigInput;
use pool_interface::types::reserve_data::ReserveData;
use pool_interface::types::user_config::UserConfiguration;
use pool_interface::types::{base_asset_config::BaseAssetConfig, pause_info::PauseInfo};
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
    BaseAsset,
    Reserves,
    ReserveAssetKey(Address),
    ReserveTimestampWindow,
    IRParams,
    UserAssetsLimit,
    UserConfig(Address),
    PriceFeed(Address),
    Pause,
    FlashLoanFee,
    MinPositionAmounts,
    STokenUnderlyingBalance(Address),
    TokenSupply(Address),
    TokenBalance(Address, Address),
    InitialHealth,
    LiquidationProtocolFee,
    ProtocolFeeVault(Address),
    Owners(Permission),
}

pub fn write_ir_params(env: &Env, ir_params: &IRParams) {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage().instance().set(&DataKey::IRParams, ir_params);
}

pub fn read_ir_params(env: &Env) -> Result<IRParams, Error> {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage()
        .instance()
        .get(&DataKey::IRParams)
        .ok_or(Error::Uninitialized)
}

pub fn read_reserve(env: &Env, asset: &Address) -> Result<ReserveData, Error> {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage()
        .instance()
        .get(&DataKey::ReserveAssetKey(asset.clone()))
        .ok_or(Error::Uninitialized)
}

pub fn write_reserve(env: &Env, asset: &Address, reserve_data: &ReserveData) {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    let asset_key: DataKey = DataKey::ReserveAssetKey(asset.clone());
    env.storage().instance().set(&asset_key, reserve_data);
}

pub fn has_reserve(env: &Env, asset: &Address) -> bool {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage()
        .instance()
        .has(&DataKey::ReserveAssetKey(asset.clone()))
}

pub fn read_reserves(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage()
        .instance()
        .get(&DataKey::Reserves)
        .unwrap_or(vec![env])
}

pub fn read_reserve_timestamp_window(env: &Env) -> u64 {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage()
        .instance()
        .get(&DataKey::ReserveTimestampWindow)
        .unwrap_or(20)
}

pub fn write_reserve_timestamp_window(env: &Env, window: u64) {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage()
        .instance()
        .set(&DataKey::ReserveTimestampWindow, &window);
}

pub fn write_reserves(env: &Env, reserves: &Vec<Address>) {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

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
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    let data_key = DataKey::PriceFeed(asset.clone());

    env.storage()
        .instance()
        .get(&data_key)
        .ok_or(Error::Uninitialized)
}

pub fn write_price_feeds(env: &Env, inputs: &Vec<PriceFeedConfigInput>) {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    for input in inputs.iter() {
        let data_key = DataKey::PriceFeed(input.asset.clone());

        let config = PriceFeedConfig {
            asset_decimals: input.asset_decimals,
            min_sanity_price_in_base: input.min_sanity_price_in_base,
            max_sanity_price_in_base: input.max_sanity_price_in_base,
            feeds: input.feeds,
        };

        env.storage().instance().set(&data_key, &config);
    }
}

pub fn write_base_asset(env: &Env, config: &BaseAssetConfig) {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    let data_key = DataKey::BaseAsset;

    env.storage().instance().set(&data_key, config);
}

pub fn read_base_asset(env: &Env) -> Result<BaseAssetConfig, Error> {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    let data_key = DataKey::BaseAsset;

    env.storage()
        .instance()
        .get(&data_key)
        .ok_or(Error::Uninitialized)
}

pub fn write_initial_health(env: &Env, value: u32) {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    let data_key = DataKey::InitialHealth;

    env.storage().instance().set(&data_key, &value);
}

pub fn read_initial_health(env: &Env) -> Result<u32, Error> {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    let data_key = DataKey::InitialHealth;

    env.storage()
        .instance()
        .get(&data_key)
        .ok_or(Error::Uninitialized)
}

pub fn read_user_assets_limit(env: &Env) -> u32 {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage()
        .instance()
        .get(&DataKey::UserAssetsLimit)
        .unwrap_or(3)
}

pub fn write_user_assets_limit(env: &Env, user_assets_limit: u32) {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage()
        .instance()
        .set(&DataKey::UserAssetsLimit, &user_assets_limit);
}

pub fn read_min_position_amounts(env: &Env) -> (i128, i128) {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage()
        .instance()
        .get(&DataKey::MinPositionAmounts)
        .unwrap_or((0, 0))
}

pub fn write_min_position_amounts(env: &Env, min_collat_amount: i128, min_debt_amount: i128) {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage().instance().set(
        &DataKey::MinPositionAmounts,
        &(min_collat_amount, min_debt_amount),
    );
}

pub fn read_pause_info(env: &Env) -> Result<PauseInfo, Error> {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage()
        .instance()
        .get(&DataKey::Pause)
        .ok_or(Error::Uninitialized)
}

pub fn write_pause_info(env: &Env, value: PauseInfo) {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage().instance().set(&DataKey::Pause, &value);
}

pub fn write_flash_loan_fee(env: &Env, fee: u32) {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage().instance().set(&DataKey::FlashLoanFee, &fee);
}

pub fn read_flash_loan_fee(env: &Env) -> u32 {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage()
        .instance()
        .get(&DataKey::FlashLoanFee)
        .unwrap()
}

fn write_stoken_underlying_balance(
    env: &Env,
    s_token_address: &Address,
    total_supply: i128,
) -> Result<(), Error> {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    assert_with_error!(env, !total_supply.is_negative(), Error::MustBeNonNegative);

    let data_key = DataKey::STokenUnderlyingBalance(s_token_address.clone());
    env.storage().instance().set(&data_key, &total_supply);

    Ok(())
}

pub fn read_stoken_underlying_balance(env: &Env, s_token_address: &Address) -> i128 {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    let data_key = DataKey::STokenUnderlyingBalance(s_token_address.clone());
    env.storage().instance().get(&data_key).unwrap_or(0i128)
}

pub fn add_stoken_underlying_balance(
    env: &Env,
    s_token_address: &Address,
    amount: i128,
) -> Result<i128, Error> {
    let mut total_supply = read_stoken_underlying_balance(env, s_token_address);

    total_supply = total_supply
        .checked_add(amount)
        .ok_or(Error::MathOverflowError)?;

    write_stoken_underlying_balance(env, s_token_address, total_supply)?;

    Ok(total_supply)
}

pub fn read_token_total_supply(env: &Env, token: &Address) -> i128 {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    let key = DataKey::TokenSupply(token.clone());
    env.storage().instance().get(&key).unwrap_or(0i128)
}

pub fn write_token_total_supply(
    env: &Env,
    token: &Address,
    total_supply: i128,
) -> Result<(), Error> {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

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

pub fn read_liquidation_protocol_fee(env: &Env) -> u32 {
    let key = DataKey::LiquidationProtocolFee;
    let value = env.storage().instance().get(&key);

    if value.is_some() {
        env.storage()
            .instance()
            .extend_ttl(LOW_USER_DATA_BUMP_LEDGERS, HIGH_USER_DATA_BUMP_LEDGERS);
    }

    value.unwrap_or(0)
}

pub fn write_liquidation_protocol_fee(env: &Env, value: u32) {
    let key = DataKey::LiquidationProtocolFee;

    env.storage().instance().set(&key, &value);
    env.storage()
        .instance()
        .extend_ttl(LOW_USER_DATA_BUMP_LEDGERS, HIGH_USER_DATA_BUMP_LEDGERS);
}

pub fn read_protocol_fee_vault(env: &Env, asset: &Address) -> i128 {
    let key = DataKey::ProtocolFeeVault(asset.clone());
    let value = env.storage().persistent().get(&key);

    if value.is_some() {
        env.storage().persistent().extend_ttl(
            &key,
            LOW_USER_DATA_BUMP_LEDGERS,
            HIGH_USER_DATA_BUMP_LEDGERS,
        );
    }

    value.unwrap_or(0)
}

pub fn write_protocol_fee_vault(env: &Env, asset: &Address, balance: i128) {
    assert_with_error!(env, !balance.is_negative(), Error::MustBeNonNegative);
    let key = DataKey::ProtocolFeeVault(asset.clone());

    env.storage().persistent().set(&key, &balance);
    env.storage().persistent().extend_ttl(
        &key,
        LOW_USER_DATA_BUMP_LEDGERS,
        HIGH_USER_DATA_BUMP_LEDGERS,
    );
}

pub fn add_protocol_fee_vault(env: &Env, asset: &Address, amount: i128) -> Result<(), Error> {
    let mut balance = read_protocol_fee_vault(env, asset);
    balance = balance
        .checked_add(amount)
        .ok_or(Error::MathOverflowError)?;

    write_protocol_fee_vault(env, asset, balance);

    Ok(())
}

pub fn write_permission_owners(env: &Env, owners: &Vec<Address>, permission: &Permission) {
    let key = DataKey::Owners(permission.clone());

    env.storage().persistent().set(&key, owners);
    env.storage().persistent().extend_ttl(
        &key,
        LOW_USER_DATA_BUMP_LEDGERS,
        HIGH_USER_DATA_BUMP_LEDGERS,
    );
}

pub fn read_permission_owners(env: &Env, permission: &Permission) -> Vec<Address> {
    let key = DataKey::Owners(permission.clone());

    let mb_vec = env.storage().persistent().get(&key);

    if mb_vec.is_some() {
        env.storage().persistent().extend_ttl(
            &key,
            LOW_INSTANCE_BUMP_LEDGERS,
            HIGH_INSTANCE_BUMP_LEDGERS,
        );
    }

    mb_vec.unwrap_or(vec![env])
}
