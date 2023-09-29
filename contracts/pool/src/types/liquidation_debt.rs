use pool_interface::types::reserve_data::ReserveData;
use soroban_sdk::Address;

#[derive(Debug, Clone)]
pub struct LiquidationDebt {
    pub asset: Address,
    pub reserve_data: ReserveData,
    pub debt_token_balance: i128,
    pub asset_price: i128,
    pub debt_coeff: i128,
    pub compounded_debt: i128,
}
