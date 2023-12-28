use soroban_sdk::{contracttype, Address};

use crate::types::oracle_asset::OracleAsset;

use super::price_feed_input::PriceFeedInput;

#[derive(Clone)]
#[contracttype]
pub struct PriceFeedConfig {
    pub feed: Address,
    pub feed_asset: OracleAsset,
    pub feed_decimals: u32,
    pub asset_decimals: u32,
}

impl PriceFeedConfig {
    pub fn new(input: &PriceFeedInput) -> Self {
        Self {
            feed: input.feed.clone(),
            feed_asset: input.feed_asset.clone(),
            feed_decimals: input.feed_decimals,
            asset_decimals: input.asset_decimals,
        }
    }
}
