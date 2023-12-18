use soroban_sdk::{contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Price(Address),
}

pub fn write_init_data(env: &Env, asset: Address, price: i128) {
    env.storage().instance().set(&DataKey::Price(asset), &price);
}

pub fn read_asset_price(env: &Env, asset: Address) -> Option<i128> {
    let data_key = DataKey::Price(asset);

    if !env.storage().instance().has(&data_key) {
        return None;
    }

    Some(env.storage().instance().get(&data_key).unwrap())
}
