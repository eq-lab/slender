use soroban_sdk::{contracttype, Address};

use super::oracle_asset::OracleAsset;

#[derive(Clone)]
#[contracttype]
pub struct PriceFeedInput {
    pub asset: Address,
    pub feed_asset: OracleAsset,
    pub asset_decimals: u32,
    pub feed: Address,
    pub feed_decimals: u32,
}
