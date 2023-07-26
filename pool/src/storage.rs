use crate::Error;
use pool_interface::{IRParams, ReserveData, UserConfiguration};
use soroban_sdk::{contracttype, vec, Address, Env, Vec};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    ReserveAssetKey(Address),
    Reserves,
    Treasury,
    IRParams,
    UserConfig(Address),
    PriceFeed(Address),
    Pause,
    STokenUnderlyingSupply(Address),
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().has(&DataKey::Admin)
}

pub fn write_admin(env: &Env, admin: Address) {
    env.storage().set(&DataKey::Admin, &admin);
}

pub fn read_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .get(&DataKey::Admin)
        .ok_or(Error::Uninitialized)?
        .unwrap()
}

pub fn write_ir_params(env: &Env, ir_params: &IRParams) {
    env.storage().set(&DataKey::IRParams, ir_params);
}

pub fn read_ir_params(env: &Env) -> Result<IRParams, Error> {
    env.storage()
        .get(&DataKey::IRParams)
        .ok_or(Error::Uninitialized)?
        .unwrap()
}

pub fn read_reserve(env: &Env, asset: Address) -> Result<ReserveData, Error> {
    let reserve_data = env
        .storage()
        .get(&DataKey::ReserveAssetKey(asset))
        .ok_or(Error::NoReserveExistForAsset)?
        .unwrap();
    Ok(reserve_data)
}

pub fn write_reserve(env: &Env, asset: Address, reserve_data: &ReserveData) {
    let asset_key = DataKey::ReserveAssetKey(asset);
    env.storage().set(&asset_key, reserve_data);
}

pub fn has_reserve(env: &Env, asset: Address) -> bool {
    env.storage().has(&DataKey::ReserveAssetKey(asset))
}

pub fn read_reserves(env: &Env) -> Vec<Address> {
    env.storage()
        .get(&DataKey::Reserves)
        .unwrap_or(Ok(vec![env]))
        .unwrap()
}

pub fn write_reserves(env: &Env, reserves: &Vec<Address>) {
    env.storage().set(&DataKey::Reserves, reserves);
}

pub fn read_user_config(env: &Env, user: Address) -> Result<UserConfiguration, Error> {
    env.storage()
        .get(&DataKey::UserConfig(user))
        .ok_or(Error::UserConfigNotExists)?
        .unwrap()
}

pub fn write_user_config(env: &Env, user: Address, config: &UserConfiguration) {
    env.storage().set(&DataKey::UserConfig(user), config);
}

pub fn read_price_feed(env: &Env, asset: Address) -> Result<Address, Error> {
    let data_key = DataKey::PriceFeed(asset);

    env.storage()
        .get(&data_key)
        .ok_or(Error::NoPriceFeed)?
        .unwrap()
}

pub fn write_price_feed(env: &Env, feed: Address, assets: &Vec<Address>) {
    for asset in assets.iter() {
        let data_key = DataKey::PriceFeed(asset.unwrap());
        env.storage().set(&data_key, &feed);
    }
}

pub fn paused(env: &Env) -> bool {
    env.storage()
        .get(&DataKey::Pause)
        .unwrap_or(Ok(false))
        .unwrap()
}

pub fn write_pause(env: &Env, value: bool) {
    env.storage().set(&DataKey::Pause, &value);
}

pub fn write_treasury(e: &Env, treasury: &Address) {
    e.storage().set(&DataKey::Treasury, treasury);
}

pub fn read_treasury(e: &Env) -> Address {
    e.storage().get_unchecked(&DataKey::Treasury).unwrap()
}

pub fn write_stoken_underlying_supply(e: &Env, s_token_address: Address, total_supply: i128) {
    let data_key = DataKey::STokenUnderlyingSupply(s_token_address);
    e.storage().set(&data_key, &total_supply);
}

pub fn read_stoken_underlying_supply(e: &Env, s_token_address: Address) -> i128 {
    let data_key = DataKey::STokenUnderlyingSupply(s_token_address);
    e.storage().get_unchecked(&data_key).unwrap_or(0)
}

pub fn add_stoken_underlying_supply(
    env: &Env,
    s_token_address: Address,
    amount: i128,
) -> Result<i128, Error> {
    if amount < 0 {
        return Err(Error::MustBePositive);
    }

    let total_supply = read_stoken_underlying_supply(&env, s_token_address.clone())
        .checked_add(amount)
        .ok_or(Error::MathOverflowError)?;

    write_stoken_underlying_supply(env, s_token_address.clone(), total_supply);

    Ok(total_supply)
}

pub fn sub_stoken_underlying_supply(
    env: &Env,
    s_token_address: Address,
    amount: i128,
) -> Result<i128, Error> {
    if amount < 0 {
        return Err(Error::MustBePositive);
    }

    let total_supply = read_stoken_underlying_supply(&env, s_token_address.clone())
        .checked_sub(amount)
        .ok_or(Error::MathOverflowError)?;

    write_stoken_underlying_supply(env, s_token_address.clone(), total_supply);

    Ok(total_supply)
}
