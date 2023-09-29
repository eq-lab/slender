use soroban_sdk::{vec, Env, Vec};

use super::liquidation_collateral::LiquidationCollateral;
use super::liquidation_debt::LiquidationDebt;

#[derive(Debug, Clone)]
pub struct LiquidationData {
    pub debt_to_cover_in_xlm: i128,
    pub debt_to_cover: Option<LiquidationDebt>,
    pub collateral_to_receive: Vec<LiquidationCollateral>,
}

impl LiquidationData {
    pub fn default(env: &Env) -> Self {
        Self {
            debt_to_cover_in_xlm: Default::default(),
            debt_to_cover: None,
            collateral_to_receive: vec![env],
        }
    }
}
