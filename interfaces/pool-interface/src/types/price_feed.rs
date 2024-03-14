use soroban_sdk::{contracttype, Address};

use super::oracle_asset::OracleAsset;
use super::timestamp_precision::TimestampPrecision;

#[derive(Clone)]
#[contracttype]
pub struct PriceFeed {
    pub feed: Address,
    pub feed_asset: OracleAsset,
    pub feed_decimals: u32,
    pub twap_records: u32,
    pub timestamp_precision: TimestampPrecision,
}
