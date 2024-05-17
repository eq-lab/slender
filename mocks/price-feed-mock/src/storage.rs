use price_feed_interface::types::asset::Asset;
use price_feed_interface::types::price_data::PriceData;
use soroban_sdk::{contracttype, Address, Env, Symbol, Vec};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    StellarPrices(Address),
    OtherPrices(Symbol),
}

pub fn write_init_data(env: &Env, asset: &Asset, prices: Vec<PriceData>) {
    let data_key = match asset {
        Asset::Stellar(asset) => DataKey::StellarPrices(asset.clone()),
        Asset::Other(asset) => DataKey::OtherPrices(asset.clone()),
    };

    env.storage().instance().set(&data_key, &prices);
}

pub fn read_prices(env: &Env, asset: &Asset) -> Option<Vec<PriceData>> {
    let data_key = match asset {
        Asset::Stellar(asset) => DataKey::StellarPrices(asset.clone()),
        Asset::Other(asset) => DataKey::OtherPrices(asset.clone()),
    };

    if !env.storage().instance().has(&data_key) {
        return None; //@audit 0 read
    } 

    Some(env.storage().instance().get(&data_key).unwrap()) //@audit 1 read
} //@audit 0 or 1 read
