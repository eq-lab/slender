use super::liquidation_collateral::LiquidationCollateral;
use super::liquidation_debt::LiquidationDebt;

#[derive(Debug, Clone, Default)]
pub struct LiquidationData {
    pub debt_to_cover_in_base: i128,
    pub debt_to_cover: Option<LiquidationDebt>,
    pub collat_to_receive: Option<LiquidationCollateral>,
}
