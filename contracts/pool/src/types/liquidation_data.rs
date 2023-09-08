use pool_interface::types::reserve_data::ReserveData;
use soroban_sdk::{vec, Address, Env, Vec};

use super::liquidation_collateral::LiquidationCollateral;

#[derive(Debug, Clone)]
pub struct LiquidationData {
    pub total_debt_with_penalty_in_xlm: i128,
    /// asset, reserve data, compounded debt, debtToken balance
    pub debt_to_cover: Vec<(Address, ReserveData, i128, i128)>,
    pub collateral_to_receive: Vec<LiquidationCollateral>,
}

impl LiquidationData {
    pub fn default(env: &Env) -> Self {
        Self {
            total_debt_with_penalty_in_xlm: Default::default(),
            debt_to_cover: vec![env],
            collateral_to_receive: vec![env],
        }
    }
}
