use pool_interface::types::reserve_data::ReserveData;
use soroban_sdk::{contracttype, Address};

#[derive(Debug, Clone)]
#[contracttype]
pub struct LiquidationDebt {
    pub asset: Address,
    pub reserve_data: ReserveData,
    pub debt_token_balance: i128,
    pub debt_coeff: i128,
    pub compounded_debt: i128,
}
