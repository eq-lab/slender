use soroban_sdk::contracttype;

#[contracttype]
#[derive(Debug, Clone)]
pub struct ReserveConfiguration {
    pub is_active: bool,
    pub borrowing_enabled: bool,
    pub liquidity_cap: i128,
    pub pen_order: u32,
    pub util_cap: u32,
    /// Specifies what fraction of the underlying asset counts toward
    /// the portfolio collateral value [0%, 100%].
    pub discount: u32,
}

impl ReserveConfiguration {
    pub(crate) fn default() -> Self {
        Self {
            liquidity_cap: Default::default(),
            pen_order: Default::default(),
            util_cap: Default::default(),
            is_active: true,
            borrowing_enabled: false,
            discount: Default::default(),
        }
    }
}
