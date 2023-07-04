use soroban_sdk::{contracttype, Address, Env, String};
use soroban_token_sdk::{TokenMetadata, TokenUtils};

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
    Balance(Address),
    State(Address), //TODO: bad naming: Authorized/ Not authorized
    InterestIndex(Address),
    Pool,
    UnderlyingAsset,
    Treasury,
    TotalSupply,
}

pub fn read_pool(e: &Env) -> Address {
    e.storage().persistent().get(&DataKey::Pool).unwrap()
}

pub fn write_pool(e: &Env, id: &Address) {
    e.storage().persistent().set(&DataKey::Pool, id);
}

pub fn has_pool(e: &Env) -> bool {
    e.storage().persistent().has(&DataKey::Pool)
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

pub fn write_treasury(e: &Env, treasury: &Address) {
    e.storage().persistent().set(&DataKey::Treasury, treasury);
}

pub fn read_treasury(e: &Env) -> Address {
    e.storage().persistent().get(&DataKey::Treasury).unwrap()
}

pub fn read_allowance(e: &Env, from: Address, spender: Address) -> i128 {
    let key = DataKey::Allowance(AllowanceDataKey { from, spender });
    e.storage().persistent().get(&key).unwrap_or(0)
}

pub fn write_allowance(e: &Env, from: Address, spender: Address, amount: i128) {
    let key = DataKey::Allowance(AllowanceDataKey { from, spender });
    e.storage().persistent().set(&key, &amount);
}

pub fn read_balance(e: &Env, addr: Address) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::Balance(addr))
        .unwrap_or(0)
}

pub fn write_balance(e: &Env, addr: Address, amount: i128) {
    e.storage()
        .persistent()
        .set(&DataKey::Balance(addr), &amount);
}

pub fn is_authorized(e: &Env, addr: Address) -> bool {
    e.storage()
        .persistent()
        .get(&DataKey::State(addr))
        .unwrap_or(true)
}

pub fn read_total_supply(e: &Env) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::TotalSupply)
        .unwrap_or(0)
}

pub fn write_total_supply(e: &Env, val: i128) {
    e.storage().persistent().set(&DataKey::TotalSupply, &val);
}

pub fn read_decimal(e: &Env) -> u32 {
    let util = TokenUtils::new(e);
    util.get_metadata().decimal
}

pub fn read_name(e: &Env) -> String {
    let util = TokenUtils::new(e);
    util.get_metadata().name
}

pub fn read_symbol(e: &Env) -> String {
    let util = TokenUtils::new(e);
    util.get_metadata().symbol
}

pub fn write_metadata(e: &Env, metadata: TokenMetadata) {
    let util = TokenUtils::new(e);
    util.set_metadata(&metadata);
}
