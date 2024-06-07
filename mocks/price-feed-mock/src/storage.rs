use price_feed_interface::types::asset::Asset;
use price_feed_interface::types::price_data::PriceData;
use soroban_sdk::{contracttype, Address, Env, Symbol, Vec};

use crate::extensions::env_extensions::EnvExtensions;

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    StellarPrices(Address),
    OtherPrices(Symbol),
}

const ASSET_COUNTER_KEY: &str = "asset_counter";

pub fn write_init_data(env: &Env, asset: &Asset, prices: Vec<PriceData>) {
    let data_key = match asset {
        Asset::Stellar(asset) => DataKey::StellarPrices(asset.clone()),
        Asset::Other(asset) => DataKey::OtherPrices(asset.clone()),
    };

    if env.get_asset_index(asset).is_none() {
        let index = read_assets_counter(env);
        match asset {
            Asset::Stellar(asset) => env.storage().instance().set(asset, &index),
            Asset::Other(symbol) => env.storage().instance().set(symbol, &index),
        };

        write_assets_counter(env, index + 1);
    }

    env.storage().instance().set(&data_key, &prices);
}

pub fn read_assets_counter(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&ASSET_COUNTER_KEY)
        .unwrap_or(0)
}

pub fn write_assets_counter(env: &Env, value: u32) {
    env.storage().instance().set(&ASSET_COUNTER_KEY, &value)
}
