use soroban_sdk::{contracttype, Address};

#[contracttype]
pub struct FlashLoanAsset {
    pub asset: Address,
    pub amount: i128,
    pub borrow: bool,
}
