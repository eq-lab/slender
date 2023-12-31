use soroban_sdk::contracttype;

#[contracttype]
#[derive(Debug, Clone)]
pub struct ReserveConfiguration {
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
    pub(crate) fn default() -> Self {
        Self {
            liq_bonus: Default::default(),
            liq_cap: Default::default(),
            util_cap: Default::default(),
            is_active: true,
            borrowing_enabled: false,
            discount: Default::default(),
        }
    }
}
