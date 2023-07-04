use crate::Error;
use pool_interface::{ReserveData, UserConfiguration};
use soroban_sdk::{contracttype, vec, Address, Env, Vec};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    ReserveAssetKey(Address),
    Reserves,
    UserConfig(Address),
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

pub fn read_user_config(env: &Env, user: Address) -> Option<UserConfiguration> {
    env.storage().persistent().get(&DataKey::UserConfig(user))
}

pub fn write_user_config(env: &Env, user: Address, config: &UserConfiguration) {
    env.storage()
        .persistent()
        .set(&DataKey::UserConfig(user), config);
}
