use soroban_sdk::{contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Pool,
    ShouldFail,
}

pub fn write_pool(env: &Env, pool: &Address) {
    env.storage().instance().set(&DataKey::Pool, &pool);
}

pub fn read_pool(env: &Env) -> Address {
    env.storage().instance().get(&DataKey::Pool).unwrap()
}

pub fn write_should_fail(env: &Env, should_fail: bool) {
    env.storage()
        .instance()
        .set(&DataKey::ShouldFail, &should_fail);
}

pub fn read_should_fail(env: &Env) -> bool {
    env.storage().instance().get(&DataKey::ShouldFail).unwrap()
}
