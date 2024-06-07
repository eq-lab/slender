#![deny(warnings)]
#![no_std]

mod extensions;
mod storage;
mod types;

use crate::storage::*;
use extensions::env_extensions::EnvExtensions;
use price_feed_interface::types::asset::Asset;
use price_feed_interface::types::price_data::PriceData;
use price_feed_interface::PriceFeedTrait;
use soroban_sdk::{contract, contractimpl, Env, Vec};

#[contract]
pub struct PriceFeedMock;

#[contractimpl]
impl PriceFeedTrait for PriceFeedMock {
    fn base(_env: Env) -> Asset {
        unimplemented!()
    }

    fn assets(_env: Env) -> Vec<Asset> {
        unimplemented!()
    }

    fn decimals(_env: Env) -> u32 {
        unimplemented!()
    }

    fn resolution(_env: Env) -> u32 {
        unimplemented!()
    }

    fn price(_env: Env, _asset: Asset, _timestamp: u64) -> Option<PriceData> {
        unimplemented!()
    }

    fn prices(env: Env, asset: Asset, _records: u32) -> Option<Vec<PriceData>> {
        // read_prices(&env, &asset);
        let asset_index = env.get_asset_index(&asset)?; //get the asset index to avoid multiple calls
        prices(
            &env,
            || get_price_data_by_index(&env, asset_index),
            _records,
        )
    }

    fn lastprice(_env: Env, _asset: Asset) -> Option<PriceData> {
        unimplemented!()
    }

    fn init(env: Env, asset: Asset, prices: Vec<PriceData>) {
        write_init_data(&env, &asset, prices);
    }
}

fn prices<F: Fn() -> Option<PriceData>>(
    e: &Env,
    get_price_fn: F,
    mut records: u32,
) -> Option<Vec<PriceData>> {
    // Check if the asset is valid
    let mut timestamp = obtain_record_timestamp(e);
    if timestamp == 0 {
        return None;
    }

    let mut prices = Vec::new(e);
    let resolution = e.get_resolution() as u64;

    // Limit the number of records to 20
    records = records.min(20);

    while records > 0 {
        if let Some(price) = get_price_fn() {
            prices.push_back(price);
        }

        // Decrement records counter in every iteration
        records -= 1;

        // if timestamp < resolution {
        //     break;
        // }
        timestamp -= resolution;
    }

    if prices.is_empty() {
        None
    } else {
        Some(prices)
    }
}

fn get_price_data_by_index(e: &Env, asset: u32) -> Option<PriceData> {
    let price = e.get_price(asset)?;
    Some(get_normalized_price_data(price, now(e)))
}

fn get_normalized_price_data(price: i128, timestamp: u64) -> PriceData {
    PriceData {
        price,
        timestamp: timestamp / 1000, //convert to seconds
    }
}

fn obtain_record_timestamp(e: &Env) -> u64 {
    let last_timestamp = e.get_last_timestamp();
    let ledger_timestamp = now(e);
    if last_timestamp == 0 //no prices yet
        || last_timestamp > ledger_timestamp
    //last timestamp is in the future
    // || ledger_timestamp - last_timestamp >= resolution * 2
    //last timestamp is too far in the past, so we cannot return the last price
    {
        return 0;
    }
    last_timestamp
}

fn now(e: &Env) -> u64 {
    e.ledger().timestamp() * 1000 //convert to milliseconds
}
