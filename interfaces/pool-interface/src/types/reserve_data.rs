use super::{
    collateral_params_input::CollateralParamsInput, init_reserve_input::InitReserveInput,
    reserve_configuration::ReserveConfiguration,
};
use common::FixedI128;
use soroban_sdk::{contracttype, Address, BytesN, Env};

#[derive(Debug, Clone)]
#[contracttype]
pub struct ReserveData {
    pub configuration: ReserveConfiguration,
    pub lender_ar: i128,
    pub lender_ir: i128,
    pub borrower_ar: i128,
    pub borrower_ir: i128,
    pub last_update_timestamp: u64,
    pub s_token_address: Address,
    pub debt_token_address: Address,
    /// The id of the reserve (position in the list of the active reserves).
    pub id: BytesN<1>,
}

impl ReserveData {
    pub fn new(env: &Env, input: &InitReserveInput) -> Self {
        let InitReserveInput {
            s_token_address,
            debt_token_address,
            // decimals,
        } = input;
        Self {
            lender_ar: FixedI128::ONE.into_inner(),
            lender_ir: Default::default(),
            borrower_ar: FixedI128::ONE.into_inner(),
            borrower_ir: Default::default(),
            s_token_address: s_token_address.clone(),
            debt_token_address: debt_token_address.clone(),
            // configuration: ReserveConfiguration::default(*decimals),
            configuration: ReserveConfiguration::default(),
            last_update_timestamp: env.ledger().timestamp(),
            id: zero_bytes(env), // position in reserve list
        }
    }

    pub fn update_collateral_config(&mut self, config: &CollateralParamsInput) {
        self.configuration.liq_bonus = config.liq_bonus;
        self.configuration.liq_cap = config.liq_cap;
        self.configuration.util_cap = config.util_cap;
        self.configuration.discount = config.discount;
    }

    pub fn get_id(&self) -> u8 {
        self.id.get(0).unwrap()
    }
}

fn zero_bytes<const N: usize>(env: &Env) -> BytesN<N> {
    BytesN::from_array(env, &[0; N])
}
