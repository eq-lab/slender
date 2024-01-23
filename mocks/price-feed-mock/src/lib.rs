#![deny(warnings)]
#![no_std]

mod storage;

use crate::storage::*;
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
        read_prices(&env, &asset)
    }

    fn lastprice(_env: Env, _asset: Asset) -> Option<PriceData> {
        unimplemented!()
    }

    fn init(env: Env, asset: Asset, prices: Vec<PriceData>) {
        write_init_data(&env, &asset, prices);
    }
}
