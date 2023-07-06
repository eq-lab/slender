use soroban_sdk::{contracttype, Address, Env, Bytes};
use soroban_token_sdk::{TokenMetadata, TokenUtils};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Balance(Address),
    State(Address),
    Pool,
    UnderlyingAsset,
    TotalSupply,
}

pub fn write_pool(e: &Env, id: &Address) {
    e.storage().set(&DataKey::Pool, id);
}

pub fn has_pool(e: &Env) -> bool {
    e.storage().has(&DataKey::Pool)
}

pub fn write_underlying_asset(e: &Env, asset: &Address) {
    e.storage().set(&DataKey::UnderlyingAsset, asset);
}

pub fn write_metadata(e: &Env, metadata: TokenMetadata) {
    let util = TokenUtils::new(e);
    util.set_metadata(&metadata);
}

pub fn read_balance(e: &Env, addr: Address) -> i128 {
    e.storage()
        .get(&DataKey::Balance(addr))
        .unwrap_or(Ok(0))
        .unwrap()
}

pub fn is_authorized(e: &Env, addr: Address) -> bool {
    e.storage()
        .get(&DataKey::State(addr))
        .unwrap_or(Ok(true))
        .unwrap()
}

pub fn read_pool(e: &Env) -> Address {
    e.storage().get_unchecked(&DataKey::Pool).unwrap()
}

pub fn write_authorization(e: &Env, addr: Address, is_authorized: bool) {
    let key = DataKey::State(addr);
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
        .get(&DataKey::TotalSupply)
        .unwrap_or(Ok(0))
        .unwrap()
}