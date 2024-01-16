use pool_interface::types::reserve_data::ReserveData;
use soroban_sdk::{contracttype, Address};

#[derive(Debug, Clone)]
#[contracttype]
pub struct LiquidationCollateral {
    pub asset: Address,
    pub reserve_data: ReserveData,
    pub s_token_balance: i128,
    pub collat_coeff: i128,
    pub compounded_collat: i128,
    pub collat_discount: u32,
}
