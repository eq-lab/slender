use common::rate_math::RATE_DENOMINATOR;
use soroban_sdk::{contracttype, Address, BytesN, Env};

// TODO: Liquidity (total cap) Cap, Liquidation penalty => ReserveConfigurationMap (rename ReserveConfiguration)
// TODO: add alpha, IR0 (0.02), maximum interest rate (500%), scaling coefficient (0.9) => add to ReserveData
// TODO: add method to populate the config above (add alpha, IR0, ...)

#[contracttype]
pub struct ReserveConfiguration {
    pub decimals: u32,
    pub is_active: bool,
    pub is_frozen: bool,
    pub borrowing_enabled: bool,
    pub liq_bonus: u32,
    pub liq_cap: i128,
    /// Specifies what fraction of the underlying asset counts toward
    /// the portfolio collateral value [0%, 100%].
    pub discount: u32,
}

impl ReserveConfiguration {
    fn default() -> Self {
        Self {
            liq_bonus: Default::default(),
            liq_cap: Default::default(),
            decimals: Default::default(),
            is_active: true,
            is_frozen: false,
            borrowing_enabled: false,
            discount: Default::default(),
        }
    }
}

#[contracttype]
pub struct InterestRateConfiguration {
    pub alpha: i128,
    pub rate: i128,
    pub max_rate: i128,
    pub scaling_coeff: i128,
}

impl InterestRateConfiguration {
    fn default() -> Self {
        Self {
            alpha: Default::default(),
            rate: Default::default(),
            max_rate: Default::default(),
            scaling_coeff: Default::default(),
        }
    }
}

#[allow(dead_code)]
pub struct ReserveConfigurationFlags {
    pub is_active: bool,
    pub is_frozen: bool,
    pub borrowing_enabled: bool,
}

impl ReserveConfiguration {
    pub fn get_flags(&self) -> ReserveConfigurationFlags {
        ReserveConfigurationFlags {
            is_active: self.is_active,
            is_frozen: self.is_frozen,
            borrowing_enabled: self.borrowing_enabled,
        }
    }
}

#[contracttype]
pub struct ReserveData {
    pub configuration: ReserveConfiguration,
    pub interest_rate_configuration: InterestRateConfiguration,
    pub collat_accrued_rate: i128,
    // TODO: added (add validation?, replace liquidity_index => collat_accrued_rate for collateral)
    pub debt_accrued_rate: i128,
    // TODO: added (add validation?)
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
            collat_accrued_rate: RATE_DENOMINATOR,
            debt_accrued_rate: RATE_DENOMINATOR,
            s_token_address,
            debt_token_address,
            configuration: ReserveConfiguration::default(),
            interest_rate_configuration: InterestRateConfiguration::default(),
            last_update_timestamp: Default::default(),
            id: zero_bytes(env), // position in reserve list
        }
    }

    pub fn update_state(&mut self) {
        // TODO
    }

    pub fn update_interest_rate(&mut self) {
        //TODO: not implemented
    }

    pub fn update_collateral_config(&mut self, config: CollateralParamsInput) {
        self.configuration.liq_bonus = config.liq_bonus;
        self.configuration.liq_cap = config.liq_cap;
        self.configuration.discount = config.discount;
    }

    pub fn update_interest_rate_config(&mut self, config: InterestRateConfiguration) {
        self.interest_rate_configuration.alpha = config.alpha;
        self.interest_rate_configuration.rate = config.rate;
        self.interest_rate_configuration.max_rate = config.max_rate;
        self.interest_rate_configuration.scaling_coeff = config.scaling_coeff;
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
    /// Specifies what fraction of the underlying asset counts toward
    /// the portfolio collateral value [0%, 100%].
    pub discount: u32,
}
