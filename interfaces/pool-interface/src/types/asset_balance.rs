use soroban_sdk::{contracttype, Address};

#[contracttype]
pub struct AssetBalance {
    pub asset: Address,
    pub balance: i128,
}

impl AssetBalance {
    pub fn new(asset: Address, balance: i128) -> Self {
        Self { asset, balance }
    }
}
