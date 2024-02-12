use common::FixedI128;
use soroban_sdk::{contracttype, BytesN, Env};

use super::collateral_params_input::CollateralParamsInput;
use super::reserve_configuration::ReserveConfiguration;
use super::reserve_type::ReserveType;

#[derive(Debug, Clone)]
#[contracttype]
pub struct ReserveData {
    pub configuration: ReserveConfiguration,
    pub lender_ar: i128,
    pub lender_ir: i128,
    pub borrower_ar: i128,
    pub borrower_ir: i128,
    pub last_update_timestamp: u64,
    pub reserve_type: ReserveType,
    /// The id of the reserve (position in the list of the active reserves).
    pub id: BytesN<1>,
}

impl ReserveData {
    pub fn new(env: &Env, reserve_type: ReserveType) -> Self {
        Self {
            lender_ar: FixedI128::ONE.into_inner(),
            lender_ir: Default::default(),
            borrower_ar: FixedI128::ONE.into_inner(),
            borrower_ir: Default::default(),
            reserve_type,
            configuration: ReserveConfiguration::default(),
            last_update_timestamp: env.ledger().timestamp(),
            id: zero_bytes(env), // position in reserve list
        }
    }

    pub fn update_collateral_config(&mut self, config: &CollateralParamsInput) {
        self.configuration.liquidity_cap = config.liq_cap;
        self.configuration.util_cap = config.util_cap;
        self.configuration.discount = config.discount;
        self.configuration.pen_order = config.pen_order;
    }

    pub fn get_id(&self) -> u8 {
        self.id.get(0).unwrap()
    }
}

fn zero_bytes<const N: usize>(env: &Env) -> BytesN<N> {
    BytesN::from_array(env, &[0; N])
}
