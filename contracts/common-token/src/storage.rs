use soroban_sdk::{contracttype, Address, Env, String};
use soroban_token_sdk::metadata::TokenMetadata;
use soroban_token_sdk::TokenUtils;

pub(crate) const DAY_IN_LEDGERS: u32 = 17_280;

pub(crate) const LOW_USER_DATA_BUMP_LEDGERS: u32 = 10 * DAY_IN_LEDGERS; // 20 days
pub(crate) const HIGH_USER_DATA_BUMP_LEDGERS: u32 = 20 * DAY_IN_LEDGERS; // 30 days

pub const LOW_INSTANCE_BUMP_LEDGERS: u32 = DAY_IN_LEDGERS; // 1 day
pub const HIGH_INSTANCE_BUMP_LEDGERS: u32 = 7 * DAY_IN_LEDGERS; // 7 days

#[derive(Clone)]
#[contracttype]
pub enum CommonDataKey {
    Balance(Address),
    State(Address), //TODO: bad naming: Authorized/ Not authorized
    Pool,
    TotalSupply,
}

pub fn read_pool(env: &Env) -> Address {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage()
        .instance()
        .get(&CommonDataKey::Pool)
        .expect("has pool")
}

pub fn write_pool(env: &Env, id: &Address) {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage().instance().set(&CommonDataKey::Pool, id);
}

pub fn has_pool(env: &Env) -> bool {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage().instance().has(&CommonDataKey::Pool)
}

pub fn read_balance(env: &Env, addr: Address) -> i128 {
    let key = CommonDataKey::Balance(addr);
    let balance = env.storage().persistent().get(&key);

    if balance.is_some() {
        env.storage().persistent().extend_ttl(
            &key,
            LOW_USER_DATA_BUMP_LEDGERS,
            HIGH_USER_DATA_BUMP_LEDGERS,
        );
    }

    balance.unwrap_or(0)
}

pub fn write_balance(env: &Env, addr: Address, amount: i128) {
    let key = CommonDataKey::Balance(addr);
    env.storage().persistent().set(&key, &amount);
    env.storage().persistent().extend_ttl(
        &key,
        LOW_USER_DATA_BUMP_LEDGERS,
        HIGH_USER_DATA_BUMP_LEDGERS,
    );
}

pub fn is_authorized(env: &Env, addr: Address) -> bool {
    let key = CommonDataKey::State(addr);
    let is_authorized = env.storage().persistent().get(&key);

    if is_authorized.is_some() {
        env.storage().persistent().extend_ttl(
            &key,
            LOW_USER_DATA_BUMP_LEDGERS,
            HIGH_USER_DATA_BUMP_LEDGERS,
        );
    }

    is_authorized.unwrap_or(true)
}

pub fn write_authorization(env: &Env, addr: Address, is_authorized: bool) {
    let key = CommonDataKey::State(addr);
    env.storage().persistent().set(&key, &is_authorized);
    env.storage().persistent().extend_ttl(
        &key,
        LOW_USER_DATA_BUMP_LEDGERS,
        HIGH_USER_DATA_BUMP_LEDGERS,
    );
}

pub fn read_decimal(env: &Env) -> u32 {
    let util = TokenUtils::new(env);
    util.metadata().get_metadata().decimal
}

pub fn read_name(env: &Env) -> String {
    let util = TokenUtils::new(env);
    util.metadata().get_metadata().name
}

pub fn read_symbol(env: &Env) -> String {
    let util = TokenUtils::new(env);
    util.metadata().get_metadata().symbol
}

pub fn read_total_supply(env: &Env) -> i128 {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage()
        .instance()
        .get(&CommonDataKey::TotalSupply)
        .unwrap_or(0)
}

pub fn write_total_supply(env: &Env, val: i128) {
    env.storage()
        .instance()
        .extend_ttl(LOW_INSTANCE_BUMP_LEDGERS, HIGH_INSTANCE_BUMP_LEDGERS);

    env.storage()
        .instance()
        .set(&CommonDataKey::TotalSupply, &val);
}

pub fn write_metadata(env: &Env, metadata: TokenMetadata) {
    let util = TokenUtils::new(env);
    util.metadata().set_metadata(&metadata);
}
