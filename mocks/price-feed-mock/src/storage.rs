use soroban_sdk::{contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Price(Address),
}

pub fn write_asset_price(env: &Env, asset: Address, price: i128) {
    let data_key = DataKey::Price(asset);

    env.storage().instance().set(&data_key, &price);
}

pub fn read_asset_price(env: &Env, asset: Address) -> Option<i128> {
    let data_key = DataKey::Price(asset);

    if !env.storage().instance().has(&data_key) {
        return None;
    }

    Some(env.storage().instance().get(&data_key).unwrap())
}
