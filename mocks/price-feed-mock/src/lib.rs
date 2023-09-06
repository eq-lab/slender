#![deny(warnings)]
#![no_std]

mod constants;
mod storage;

use crate::storage::*;
use price_feed_interface::{types::price_data::PriceData, PriceFeedTrait};
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
        constants::DECIMALS
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
        let price = read_asset_price(&env, asset)
            .or(10i128.checked_pow(constants::DECIMALS))
            .unwrap();

        Some(PriceData {
            price,
            timestamp: env.ledger().timestamp(),
        })
    }

    fn set_price(env: Env, asset: Address, price: i128) {
        write_asset_price(&env, asset, price);
    }
}
