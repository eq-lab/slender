use soroban_sdk::{contracttype, Address};

use crate::types::asset_balance::AssetBalance;

#[contracttype]
pub struct MintBurn {
    pub asset_balance: AssetBalance,
    pub mint: bool,
    pub who: Address,
}

impl MintBurn {
    pub fn new(asset_balance: AssetBalance, mint: bool, who: Address) -> Self {
        Self {
            asset_balance,
            mint,
            who,
        }
    }
}
