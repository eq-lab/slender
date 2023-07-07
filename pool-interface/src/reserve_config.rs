use soroban_sdk::{contracttype, Address, BytesN, Env};

#[contracttype]
pub struct ReserveConfigurationMap {
    //bit 0-15: LTV
    pub ltv: u32,
    //bit 16-31: Liq. threshold
    pub liq_threshold: u32,
    //bit 32-47: Liq. bonus
    pub liq_bonus: u32,
    //bit 48-55: Decimals
    pub decimals: u32,
    //bit 56: Reserve is active
    pub is_active: bool,
    //bit 57: reserve is frozen
    pub is_frozen: bool,
    //bit 58: borrowing is enabled
    //bit 59: stable rate borrowing enabled
    pub borrowing_enabled: bool,
    //bit 60-63: reserved
    pub reserved: BytesN<1>,
    //bit 64-79: reserve factor
    pub reserve_factor: u32,
}

impl ReserveConfigurationMap {
    fn default(env: &Env) -> Self {
        Self {
            ltv: Default::default(),
            liq_threshold: Default::default(),
            liq_bonus: Default::default(),
            decimals: Default::default(),
            is_active: true,
            is_frozen: false,
            borrowing_enabled: false,
            reserved: zero_bytes(env),
            reserve_factor: Default::default(),
        }
    }
}

#[allow(dead_code)]
pub struct ReserveConfigurationFlags {
    pub is_active: bool,
    pub is_frozen: bool,
    pub borrowing_enabled: bool,
}

impl ReserveConfigurationMap {
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
    //stores the reserve configuration
    pub configuration: ReserveConfigurationMap,
    //the liquidity index. Expressed in ray
    pub liquidity_index: i128,
    //variable borrow index. Expressed in ray
    pub variable_borrow_index: i128,
    //the current supply rate. Expressed in ray
    pub current_liquidity_rate: u128,
    //the current variable borrow rate. Expressed in ray
    pub current_variable_borrow_rate: u128,
    pub last_update_timestamp: u64, // u40,
    // 24 empty bits
    //tokens addresses
    pub s_token_address: Address,
    pub debt_token_address: Address,
    //the id of the reserve. Represents the position in the list of the active reserves
    pub id: BytesN<1>,
}

impl ReserveData {
    pub fn new(env: &Env, input: InitReserveInput) -> Self {
        let InitReserveInput {
            s_token_address,
            debt_token_address,
        } = input;
        Self {
            liquidity_index: common::RATE_DENOMINATOR,
            variable_borrow_index: common::RATE_DENOMINATOR,
            s_token_address,
            debt_token_address,
            configuration: ReserveConfigurationMap::default(env),
            current_liquidity_rate: Default::default(),
            current_variable_borrow_rate: Default::default(),
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

    pub fn update_collateral_config(&mut self, config: CollateralParams) {
        self.configuration.ltv = config.ltv;
        self.configuration.liq_threshold = config.liq_threshold;
        self.configuration.liq_bonus = config.liq_bonus;
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

///Collateralization parameters
#[contracttype]
#[derive(Clone, Copy)]
pub struct CollateralParams {
    ///The threshold at which loans using this asset as collateral will be considered undercollateralized
    pub liq_threshold: u32,
    ///The bonus liquidators receive to liquidate this asset. The values is always above 100%. A value of 105% means the liquidator will receive a 5% bonus
    pub liq_bonus: u32,
    ///The loan to value of the asset when used as collateral
    pub ltv: u32,
}
