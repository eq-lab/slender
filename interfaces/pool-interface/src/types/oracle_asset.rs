use price_feed_interface::types::asset::Asset;
use soroban_sdk::{contracttype, Address, Symbol};

#[contracttype]
#[derive(Debug, Clone)]
pub enum OracleAsset {
    Stellar(Address),
    Other(Symbol),
}

impl From<OracleAsset> for Asset {
    fn from(asset: OracleAsset) -> Self {
        match asset {
            OracleAsset::Stellar(address) => Asset::Stellar(address),
            OracleAsset::Other(symbol) => Asset::Other(symbol),
        }
    }
}
