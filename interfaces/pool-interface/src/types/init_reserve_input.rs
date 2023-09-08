use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone)]
pub struct InitReserveInput {
    pub s_token_address: Address,
    pub debt_token_address: Address,
}
