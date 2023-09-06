use soroban_sdk::contracttype;

/// Interest rate parameters
#[contracttype]
#[derive(Clone)]
pub struct IRParams {
    pub alpha: u32,
    pub initial_rate: u32,
    pub max_rate: u32,
    pub scaling_coeff: u32,
}
