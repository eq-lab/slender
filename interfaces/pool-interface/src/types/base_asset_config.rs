use soroban_sdk::{contracttype, Address};

#[derive(Clone)]
#[contracttype]
pub struct BaseAssetConfig {
    pub address: Address,
    pub decimals: u32,
}

impl BaseAssetConfig {
    pub fn new(asset: &Address, decimals: u32) -> Self {
        Self {
            address: asset.clone(),
            decimals,
        }
    }
}
