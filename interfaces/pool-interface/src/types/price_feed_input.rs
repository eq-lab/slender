use soroban_sdk::{contracttype, Address};

#[derive(Clone)]
#[contracttype]
pub struct PriceFeedInput {
    pub asset: Address,
    pub asset_decimals: u32,
    pub feed: Address,
    pub feed_decimals: u32,
}
