use price_feed_interface::types::asset::Asset;
use price_feed_interface::types::price_data::PriceData;
use soroban_sdk::{Env, Vec};

use crate::extensions::{
    env_extensions::{EnvExtensions, LAST_TIMESTAMP},
    u128_helper::U128Helper,
};

pub(crate) const DAY_IN_LEDGERS: u32 = 17_280;

pub const LOW_INSTANCE_BUMP_LEDGERS: u32 = 10 * DAY_IN_LEDGERS;
pub const HIGH_INSTANCE_BUMP_LEDGERS: u32 = 20 * DAY_IN_LEDGERS;

const ASSET_COUNTER_KEY: &str = "asset_counter";

pub fn write_init_data(env: &Env, asset: &Asset, prices: Vec<PriceData>) {
    let index = env.get_asset_index(asset).unwrap_or_else(|| {
        let index = read_assets_counter(env);
        match asset {
            Asset::Stellar(address) => env.storage().instance().set(address, &index),
            Asset::Other(symbol) => env.storage().instance().set(symbol, &index),
        };

        write_assets_counter(env, index + 1);

        index
    });

    let mut max_timestamp = 0;
    for i in 0..prices.len() {
        let p = prices.get(i).unwrap();
        let data_key = U128Helper::encode_price_record_key(i as u64, index);
        // let data_key = index;
        env.storage().temporary().set(&data_key, &p);
        env.storage().temporary().extend_ttl(
            &data_key,
            LOW_INSTANCE_BUMP_LEDGERS,
            HIGH_INSTANCE_BUMP_LEDGERS,
        );
        if p.timestamp > max_timestamp {
            max_timestamp = p.timestamp;
        }
    }
    env.storage()
        .instance()
        .set(&LAST_TIMESTAMP, &max_timestamp);
    env.set_prices_length(index, prices.len());
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
