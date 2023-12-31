use pool_interface::types::account_position::AccountPosition;

use super::liquidation_data::LiquidationData;

#[allow(dead_code)] //TODO: remove after full implement validate_borrow
#[derive(Debug, Clone, Default)]
pub struct AccountData {
    /// Total collateral expresed in XLM
    pub discounted_collateral: i128,
    /// Total debt expressed in XLM
    pub debt: i128,
    /// Net position value in XLM
    pub npv: i128,
    /// Liquidation data
    pub liquidation: Option<LiquidationData>,
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
