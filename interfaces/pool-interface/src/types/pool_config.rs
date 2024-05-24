use soroban_sdk::{contracttype, Address};

#[derive(Clone)]
#[contracttype]
pub struct PoolConfig {
    pub base_asset_address: Address,
    pub base_asset_decimals: u32,
    pub initial_health: u32,
    pub timestamp_window: u64,
    pub flash_loan_fee: u32,
    pub user_assets_limit: u32,
    pub min_collat_amount: i128,
    pub min_debt_amount: i128,
}
