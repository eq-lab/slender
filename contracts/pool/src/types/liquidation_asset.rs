use pool_interface::types::reserve_data::ReserveData;
use soroban_sdk::{contracttype, Address};

#[derive(Debug, Clone)]
#[contracttype]
pub struct LiquidationAsset {
    pub asset: Address,
    pub reserve: ReserveData,
    pub comp_balance: i128,
    pub lp_balance: Option<i128>,
    pub coeff: Option<i128>,
}
