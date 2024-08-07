use soroban_sdk::{contracttype, Address, Vec};

use crate::types::price_feed::PriceFeed;

#[derive(Clone)]
#[contracttype]
pub struct PriceFeedConfigInput {
    pub asset: Address,
    pub asset_decimals: u32,
    pub min_sanity_price_in_base: i128,
    pub max_sanity_price_in_base: i128,
    pub feeds: Vec<PriceFeed>,
}
