use soroban_sdk::{contracttype, Address, Bytes, Env};
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
    e.storage().get_unchecked(&DataKey::Pool).unwrap()
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

pub fn read_underlying_asset(e: &Env) -> Address {
    e.storage()
        .get_unchecked(&DataKey::UnderlyingAsset)
        .unwrap()
}

pub fn write_treasury(e: &Env, treasury: &Address) {
    e.storage().set(&DataKey::Treasury, treasury);
}

pub fn read_treasury(e: &Env) -> Address {
    e.storage().get_unchecked(&DataKey::Treasury).unwrap()
}

pub fn read_allowance(e: &Env, from: Address, spender: Address) -> i128 {
    let key = DataKey::Allowance(AllowanceDataKey { from, spender });
    if let Some(allowance) = e.storage().get(&key) {
        allowance.unwrap()
    } else {
        0
    }
}

pub fn write_allowance(e: &Env, from: Address, spender: Address, amount: i128) {
    let key = DataKey::Allowance(AllowanceDataKey { from, spender });
    e.storage().set(&key, &amount);
}

pub fn read_balance(e: &Env, addr: Address) -> i128 {
    if let Some(balance) = e.storage().get(&DataKey::Balance(addr)) {
        balance.unwrap()
    } else {
        0
    }
}

pub fn write_balance(e: &Env, addr: Address, amount: i128) {
    let key = DataKey::Balance(addr);
    e.storage().set(&key, &amount);
}

pub fn is_authorized(e: &Env, addr: Address) -> bool {
    let key = DataKey::State(addr);
    if let Some(state) = e.storage().get(&key) {
        state.unwrap()
    } else {
        true
    }
}

pub fn write_authorization(e: &Env, addr: Address, is_authorized: bool) {
    let key = DataKey::State(addr);
    e.storage().set(&key, &is_authorized);
}

pub fn read_total_supply(e: &Env) -> i128 {
    e.storage()
        .get(&DataKey::TotalSupply)
        .unwrap_or(Ok(0))
        .unwrap()
}

pub fn write_total_supply(e: &Env, val: i128) {
    e.storage().set(&DataKey::TotalSupply, &val);
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

pub fn write_metadata(e: &Env, metadata: TokenMetadata) {
    let util = TokenUtils::new(e);
    util.set_metadata(&metadata);
}
