use common::FixedI128;
use soroban_sdk::{contracttype, Address, BytesN, Env};

#[contracttype]
#[derive(Debug, Clone)]
pub struct ReserveConfiguration {
    pub decimals: u32,
    pub is_active: bool,
    pub borrowing_enabled: bool,
    pub liq_bonus: u32,
    pub liq_cap: i128,
    pub util_cap: u32,
    /// Specifies what fraction of the underlying asset counts toward
    /// the portfolio collateral value [0%, 100%].
    pub discount: u32,
}

impl ReserveConfiguration {
    fn default() -> Self {
        Self {
            liq_bonus: Default::default(),
            liq_cap: Default::default(),
            util_cap: Default::default(),
            decimals: Default::default(),
            is_active: true,
            borrowing_enabled: false,
            discount: Default::default(),
        }
    }
}

/// Interest rate parameters
#[contracttype]
#[derive(Clone)]
pub struct IRParams {
    pub alpha: u32,
    pub initial_rate: u32,
    pub max_rate: u32,
    pub scaling_coeff: u32,
}

#[allow(dead_code)]
pub struct ReserveConfigurationFlags {
    pub is_active: bool,
    pub borrowing_enabled: bool,
}

impl ReserveConfiguration {
    pub fn get_flags(&self) -> ReserveConfigurationFlags {
        ReserveConfigurationFlags {
            is_active: self.is_active,
            borrowing_enabled: self.borrowing_enabled,
        }
    }
}

#[derive(Debug, Clone)]
#[contracttype]
pub struct ReserveData {
    pub configuration: ReserveConfiguration,
    pub lender_accrued_rate: i128,
    pub borrower_accrued_rate: i128,
    pub borrower_ir: i128,
    pub lender_ir: i128,
    pub last_update_timestamp: u64,
    pub s_token_address: Address,
    pub debt_token_address: Address,
    /// The id of the reserve (position in the list of the active reserves).
    pub id: BytesN<1>,
}

impl ReserveData {
    pub fn new(env: &Env, input: InitReserveInput) -> Self {
        let InitReserveInput {
            s_token_address,
            debt_token_address,
        } = input;
        Self {
            lender_accrued_rate: FixedI128::ONE.into_inner(),
            borrower_accrued_rate: FixedI128::ONE.into_inner(),
            borrower_ir: Default::default(),
            lender_ir: Default::default(),
            s_token_address,
            debt_token_address,
            configuration: ReserveConfiguration::default(),
            last_update_timestamp: env.ledger().timestamp(),
            id: zero_bytes(env), // position in reserve list
        }
    }

    pub fn update_collateral_config(&mut self, config: CollateralParamsInput) {
        self.configuration.liq_bonus = config.liq_bonus;
        self.configuration.liq_cap = config.liq_cap;
        self.configuration.util_cap = config.util_cap;
        self.configuration.discount = config.discount;
    }

    pub fn get_id(&self) -> u8 {
        self.id.get(0).unwrap()
    }
}

#[contracttype]
#[derive(Clone)]
pub struct InitReserveInput {
    pub s_token_address: Address,
    pub debt_token_address: Address,
}

fn zero_bytes<const N: usize>(env: &Env) -> BytesN<N> {
    BytesN::from_array(env, &[0; N])
}

/// Collateralization parameters
#[contracttype]
#[derive(Clone, Copy)]
pub struct CollateralParamsInput {
    /// The bonus liquidators receive to liquidate this asset. The values is always above 100%. A value of 105% means the liquidator will receive a 5% bonus
    pub liq_bonus: u32,
    /// The total amount of an asset the protocol accepts into the market.
    pub liq_cap: i128,
    pub util_cap: u32,
    /// Specifies what fraction of the underlying asset counts toward
    /// the portfolio collateral value [0%, 100%].
    pub discount: u32,
}
