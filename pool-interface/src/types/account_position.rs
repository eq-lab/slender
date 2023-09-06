use soroban_sdk::contracttype;

#[contracttype]
pub struct AccountPosition {
    pub discounted_collateral: i128,
    pub debt: i128,
    pub npv: i128,
}
