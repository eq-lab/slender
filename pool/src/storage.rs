use crate::Error;
use pool_interface::{IRParams, ReserveData, UserConfiguration};
use soroban_sdk::{assert_with_error, contracttype, vec, Address, Env, Vec};

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
    env.storage().instance().has(&DataKey::Admin)
}

pub fn write_admin(env: &Env, admin: Address) {
    env.storage().instance().set(&DataKey::Admin, &admin);
}

pub fn read_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::Uninitialized)
}

pub fn write_ir_params(env: &Env, ir_params: &IRParams) {
    env.storage().instance().set(&DataKey::IRParams, ir_params);
}

pub fn read_ir_params(env: &Env) -> Result<IRParams, Error> {
    env.storage()
        .instance()
        .get(&DataKey::IRParams)
        .ok_or(Error::Uninitialized)
}

pub fn read_reserve(env: &Env, asset: Address) -> Result<ReserveData, Error> {
    env.storage()
        .instance()
        .get(&DataKey::ReserveAssetKey(asset))
        .ok_or(Error::NoReserveExistForAsset)
}

pub fn write_reserve(env: &Env, asset: Address, reserve_data: &ReserveData) {
    let asset_key: DataKey = DataKey::ReserveAssetKey(asset);
    env.storage().instance().set(&asset_key, reserve_data);
}

pub fn has_reserve(env: &Env, asset: Address) -> bool {
    env.storage()
        .instance()
        .has(&DataKey::ReserveAssetKey(asset))
}

pub fn read_reserves(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::Reserves)
        .unwrap_or(vec![env])
}

pub fn write_reserves(env: &Env, reserves: &Vec<Address>) {
    env.storage().instance().set(&DataKey::Reserves, reserves);
}

pub fn read_user_config(env: &Env, user: Address) -> Result<UserConfiguration, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::UserConfig(user))
        .ok_or(Error::UserConfigNotExists)
}

pub fn write_user_config(env: &Env, user: Address, config: &UserConfiguration) {
    env.storage()
        .persistent()
        .set(&DataKey::UserConfig(user), config);
}

pub fn read_price_feed(env: &Env, asset: Address) -> Result<Address, Error> {
    let data_key = DataKey::PriceFeed(asset);

    env.storage()
        .instance()
        .get(&data_key)
        .ok_or(Error::NoPriceFeed)
}

pub fn write_price_feed(env: &Env, feed: Address, assets: &Vec<Address>) {
    for asset in assets.iter() {
        let data_key = DataKey::PriceFeed(asset);
        env.storage().instance().set(&data_key, &feed);
    }
}

pub fn paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::Pause)
        .unwrap_or(false)
}

pub fn write_pause(env: &Env, value: bool) {
    env.storage().instance().set(&DataKey::Pause, &value);
}

pub fn write_treasury(e: &Env, treasury: &Address) {
    e.storage().instance().set(&DataKey::Treasury, treasury);
}

pub fn read_treasury(e: &Env) -> Address {
    e.storage().instance().get(&DataKey::Treasury).unwrap()
}

pub fn write_stoken_underlying_balance(
    env: &Env,
    s_token_address: &Address,
    total_supply: i128,
) -> Result<(), Error> {
    assert_with_error!(env, !total_supply.is_negative(), Error::MustBePositive);

    let data_key = DataKey::STokenUnderlyingSupply(s_token_address.clone());
    env.storage().instance().set(&data_key, &total_supply);

    Ok(())
}

pub fn read_stoken_underlying_balance(env: &Env, s_token_address: &Address) -> i128 {
    let data_key = DataKey::STokenUnderlyingSupply(s_token_address.clone());
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
