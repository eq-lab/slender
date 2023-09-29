#![deny(warnings)]
#![no_std]

mod storage;

use crate::storage::*;
use price_feed_interface::{PriceData, PriceFeedTrait};
use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

#[contract]
pub struct PriceFeedMock;

#[contractimpl]
impl PriceFeedTrait for PriceFeedMock {
    fn base(_env: Env) -> Address {
        unimplemented!()
    }

    fn assets(_env: Env) -> Vec<Address> {
        unimplemented!()
    }

    fn decimals(_env: Env) -> u32 {
        unimplemented!()
    }

    fn resolution(_env: Env) -> u32 {
        unimplemented!()
    }

    fn price(_env: Env, _asset: Address, _timestamp: u64) -> Option<PriceData> {
        unimplemented!()
    }

    fn prices(_env: Env, _asset: Address, _records: u32) -> Option<Vec<PriceData>> {
        unimplemented!()
    }

    fn lastprice(env: Env, asset: Address) -> Option<PriceData> {
        Some(PriceData {
            price: read_asset_price(&env, asset.clone()).unwrap(),
            timestamp: env.ledger().timestamp(),
        })
    }

    fn init(env: Env, asset: Address, price: i128) {
        write_init_data(&env, asset, price);
    }
}
