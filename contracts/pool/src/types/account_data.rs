use pool_interface::types::account_position::AccountPosition;
use soroban_sdk::Vec;

use super::liquidation_asset::LiquidationAsset;

#[derive(Debug, Clone, Default)]
pub struct AccountData {
    pub discounted_collateral: i128,
    pub debt: i128,
    pub npv: i128,
    pub collat: Option<i128>,
    pub liq_debts: Option<Vec<LiquidationAsset>>,
    pub liq_collats: Option<Vec<LiquidationAsset>>,
}

impl AccountData {
    pub fn is_good_position(&self) -> bool {
        self.npv > 0
    }

    pub fn get_position(&self) -> AccountPosition {
        AccountPosition {
            discounted_collateral: self.discounted_collateral,
            debt: self.debt,
            npv: self.npv,
        }
    }
}
