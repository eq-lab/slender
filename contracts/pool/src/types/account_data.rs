use pool_interface::types::account_position::AccountPosition;
use soroban_sdk::Vec;

use super::liquidation_asset::LiquidationAsset;

#[derive(Debug, Clone, Default)]
pub struct AccountData {
    /// Total collateral expresed in XLM
    pub discounted_collateral: i128,
    /// Total debt expressed in XLM
    pub debt: i128,
    /// Net position value in XLM
    pub npv: i128,
    /// Liquidation debt ordered by max utilization
    pub liq_debts: Option<Vec<LiquidationAsset>>,
    /// Liquidation collateral ordered by liquidation_order
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
