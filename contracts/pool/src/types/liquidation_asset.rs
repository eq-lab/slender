use pool_interface::types::reserve_data::ReserveData;
use soroban_sdk::{contracttype, Address};

#[derive(Debug, Clone)]
#[contracttype]
pub struct LiquidationAsset {
    pub asset: Address,
    pub reserve: ReserveData,
    pub lp_balance: i128,
    pub comp_balance: i128,
    pub coeff: i128,
}
