use soroban_sdk::{contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
pub struct AllowanceDataKey {
    pub from: Address,
    pub spender: Address,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Allowance(AllowanceDataKey),
    UnderlyingAsset,
}

pub fn write_underlying_asset(e: &Env, asset: &Address) {
    e.storage()
        .persistent()
        .set(&DataKey::UnderlyingAsset, asset);
}

pub fn read_underlying_asset(e: &Env) -> Address {
    e.storage()
        .persistent()
        .get(&DataKey::UnderlyingAsset)
        .unwrap()
}

pub fn read_allowance(e: &Env, from: Address, spender: Address) -> i128 {
    let key = DataKey::Allowance(AllowanceDataKey { from, spender });
    e.storage().persistent().get(&key).unwrap_or(0)
}

pub fn write_allowance(e: &Env, from: Address, spender: Address, amount: i128) {
    let key = DataKey::Allowance(AllowanceDataKey { from, spender });
    e.storage().persistent().set(&key, &amount);
}
