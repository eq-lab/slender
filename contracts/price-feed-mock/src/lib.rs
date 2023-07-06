#![deny(warnings)]
#![no_std]

use soroban_sdk::{contractimpl, Address, Env, Vec};
use price_feed_interface::{PriceData, PriceFeedTrait};

pub struct PriceFeedMock;

#[contractimpl]
impl PriceFeedTrait for PriceFeedMock {
    fn base(_env: Env) -> Address {
        todo!()
    }

    fn assets(_env: Env) -> Vec<Address> {
        todo!()
    }

    fn decimals(_env: Env) -> u32 {
        todo!()
    }

    fn resolution(_env: Env) -> u32 {
        todo!()
    }

    fn price(_env: Env, _asset: Address, _timestamp: u64) -> Option<PriceData> {
        todo!()
    }

    fn prices(_env: Env, _asset: Address, _records: u32) -> Option<Vec<PriceData>> {
        todo!()
    }

    fn lastprice(env: Env, _asset: Address) -> Option<PriceData> {
        Some(PriceData {
            price: common::RATE_DENOMINATOR,
            timestamp: env.ledger().timestamp(),
        })
    }
}
