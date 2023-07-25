use soroban_sdk::{contracttype, Address, Env, String};
use soroban_token_sdk::{TokenMetadata, TokenUtils};

#[derive(Clone)]
#[contracttype]
pub enum CommonDataKey {
    Balance(Address),
    State(Address), //TODO: bad naming: Authorized/ Not authorized
    Pool,
    TotalSupply,
}

pub fn read_pool(e: &Env) -> Address {
    e.storage()
        .persistent()
        .get(&CommonDataKey::Pool)
        .expect("has pool")
}

pub fn write_pool(e: &Env, id: &Address) {
    e.storage().persistent().set(&CommonDataKey::Pool, id);
}

pub fn has_pool(e: &Env) -> bool {
    e.storage().persistent().has(&CommonDataKey::Pool)
}

pub fn read_balance(e: &Env, addr: Address) -> i128 {
    e.storage()
        .persistent()
        .get(&CommonDataKey::Balance(addr))
        .unwrap_or(0)
}

pub fn write_balance(e: &Env, addr: Address, amount: i128) {
    let key = CommonDataKey::Balance(addr);
    e.storage().persistent().set(&key, &amount);
}

pub fn is_authorized(e: &Env, addr: Address) -> bool {
    e.storage()
        .persistent()
        .get(&CommonDataKey::State(addr))
        .unwrap_or(true)
}

pub fn write_authorization(e: &Env, addr: Address, is_authorized: bool) {
    let key = CommonDataKey::State(addr);
    e.storage().persistent().set(&key, &is_authorized);
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

pub fn read_total_supply(e: &Env) -> i128 {
    e.storage()
        .persistent()
        .get(&CommonDataKey::TotalSupply)
        .unwrap_or(0)
}

pub fn write_total_supply(e: &Env, val: i128) {
    e.storage()
        .persistent()
        .set(&CommonDataKey::TotalSupply, &val);
}

pub fn write_metadata(e: &Env, metadata: TokenMetadata) {
    let util = TokenUtils::new(e);
    util.set_metadata(&metadata);
}
