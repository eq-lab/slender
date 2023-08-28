use soroban_sdk::{contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Owner,
}

pub fn write_owner(env: &Env, owner: &Address) {
    env.storage().instance().set(&DataKey::Owner, &owner);
}

pub fn read_owner(env: &Env) -> Option<Address> {
    let data_key = DataKey::Owner;

    if !env.storage().instance().has(&data_key) {
        return None;
    }

    Some(env.storage().instance().get(&data_key).unwrap())
}
