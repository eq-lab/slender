use price_feed_interface::Asset;
use soroban_sdk::{contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Price(Address),
}

pub fn write_init_data(env: &Env, asset: Asset, price: i128) {
    if let Asset::Stellar(address) = asset {
        env.storage()
            .instance()
            .set(&DataKey::Price(address), &price);
    } else {
        unimplemented!()
    }
}

pub fn read_asset_price(env: &Env, asset: Asset) -> Option<i128> {
    if let Asset::Stellar(address) = asset {
        let data_key = DataKey::Price(address);

        if !env.storage().instance().has(&data_key) {
            return None;
        }

        Some(env.storage().instance().get(&data_key).unwrap())
    } else {
        unimplemented!()
    }
}
