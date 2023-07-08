#![deny(warnings)]
#![no_std]

mod constants;

use price_feed_interface::{PriceData, PriceFeedTrait};
use soroban_sdk::{contractimpl, Address, Env, Vec};
use crate::constants::Constants;

pub struct PriceFeedMock;

#[contractimpl]
impl PriceFeedTrait for PriceFeedMock {
    fn base(_env: Env) -> Address { unimplemented!() }

    fn assets(_env: Env) -> Vec<Address> {
        unimplemented!()
    }

    fn decimals(_env: Env) -> u32 {
        Constants::DECIMALS
    }

    fn resolution(_env: Env) -> u32 {
        Constants::RESOLUTION
    }

    fn price(_env: Env, _asset: Address, _timestamp: u64) -> Option<PriceData> {
        unimplemented!()
    }

    fn prices(_env: Env, _asset: Address, _records: u32) -> Option<Vec<PriceData>> {
        unimplemented!()
    }

    fn lastprice(env: Env, _asset: Address) -> Option<PriceData> {
        Some(PriceData {
            price: common::RATE_DENOMINATOR,
            timestamp: env.ledger().timestamp(),
        })
    }
}
