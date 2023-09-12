use soroban_sdk::contracttype;

#[contracttype]
#[derive(Debug, Clone)]
pub struct ReserveConfiguration {
    pub decimals: u32,
    pub is_active: bool,
    pub is_base_asset: bool,
    pub borrowing_enabled: bool,
    pub liq_bonus: u32,
    pub liq_cap: i128,
    pub util_cap: u32,
    /// Specifies what fraction of the underlying asset counts toward
    /// the portfolio collateral value [0%, 100%].
    pub discount: u32,
}

impl ReserveConfiguration {
    pub(crate) fn default(decimals: u32) -> Self {
        Self {
            liq_bonus: Default::default(),
            liq_cap: Default::default(),
            util_cap: Default::default(),
            decimals,
            is_active: true,
            is_base_asset: false,
            borrowing_enabled: false,
            discount: Default::default(),
        }
    }
}
