use soroban_sdk::{contracttype, Address, Bytes, Env};
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
    e.storage().get_unchecked(&CommonDataKey::Pool).unwrap()
}

pub fn write_pool(e: &Env, id: &Address) {
    e.storage().set(&CommonDataKey::Pool, id);
}

pub fn has_pool(e: &Env) -> bool {
    e.storage().has(&CommonDataKey::Pool)
}

pub fn read_balance(e: &Env, addr: Address) -> i128 {
    e.storage()
        .get(&CommonDataKey::Balance(addr))
        .unwrap_or(Ok(0))
        .unwrap()
}

pub fn write_balance(e: &Env, addr: Address, amount: i128) {
    let key = CommonDataKey::Balance(addr);
    e.storage().set(&key, &amount);
}

pub fn is_authorized(e: &Env, addr: Address) -> bool {
    e.storage()
        .get(&CommonDataKey::State(addr))
        .unwrap_or(Ok(true))
        .unwrap()
}

pub fn write_authorization(e: &Env, addr: Address, is_authorized: bool) {
    let key = CommonDataKey::State(addr);
    e.storage().set(&key, &is_authorized);
}

pub fn read_decimal(e: &Env) -> u32 {
    let util = TokenUtils::new(e);
    util.get_metadata_unchecked().unwrap().decimal
}

pub fn read_name(e: &Env) -> Bytes {
    let util = TokenUtils::new(e);
    util.get_metadata_unchecked().unwrap().name
}

pub fn read_symbol(e: &Env) -> Bytes {
    let util = TokenUtils::new(e);
    util.get_metadata_unchecked().unwrap().symbol
}

pub fn read_total_supply(e: &Env) -> i128 {
    e.storage()
        .get(&CommonDataKey::TotalSupply)
        .unwrap_or(Ok(0))
        .unwrap()
}

pub fn write_total_supply(e: &Env, val: i128) {
    e.storage().set(&CommonDataKey::TotalSupply, &val);
}

pub fn write_metadata(e: &Env, metadata: TokenMetadata) {
    let util = TokenUtils::new(e);
    util.set_metadata(&metadata);
}
