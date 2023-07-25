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
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().persistent().has(&DataKey::Admin)
}

pub fn write_admin(env: &Env, admin: Address) {
    env.storage().persistent().set(&DataKey::Admin, &admin);
}

pub fn read_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Admin)
        .ok_or(Error::Uninitialized)
}

pub fn write_ir_params(env: &Env, ir_params: &IRParams) {
    env.storage()
        .persistent()
        .set(&DataKey::IRParams, ir_params);
}

pub fn read_ir_params(env: &Env) -> Result<IRParams, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::IRParams)
        .ok_or(Error::Uninitialized)
}

pub fn read_reserve(env: &Env, asset: Address) -> Result<ReserveData, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::ReserveAssetKey(asset))
        .ok_or(Error::NoReserveExistForAsset)
}

pub fn write_reserve(env: &Env, asset: Address, reserve_data: &ReserveData) {
    let asset_key = DataKey::ReserveAssetKey(asset);
    env.storage().persistent().set(&asset_key, reserve_data);
}

pub fn has_reserve(env: &Env, asset: Address) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::ReserveAssetKey(asset))
}

pub fn read_reserves(env: &Env) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::Reserves)
        .unwrap_or(vec![env])
}

pub fn write_reserves(env: &Env, reserves: &Vec<Address>) {
    env.storage().persistent().set(&DataKey::Reserves, reserves);
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
        .persistent()
        .get(&data_key)
        .ok_or(Error::NoPriceFeed)
}

pub fn write_price_feed(env: &Env, feed: Address, assets: &Vec<Address>) {
    for asset in assets.iter() {
        let data_key = DataKey::PriceFeed(asset);
        env.storage().persistent().set(&data_key, &feed);
    }
}

pub fn paused(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::Pause)
        .unwrap_or(false)
}

pub fn write_pause(env: &Env, value: bool) {
    env.storage().persistent().set(&DataKey::Pause, &value);
}

pub fn write_treasury(e: &Env, treasury: &Address) {
    e.storage().persistent().set(&DataKey::Treasury, treasury);
}

pub fn read_treasury(e: &Env) -> Address {
    e.storage().persistent().get(&DataKey::Treasury).unwrap()
}
