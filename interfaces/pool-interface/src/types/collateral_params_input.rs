use soroban_sdk::contracttype;

/// Collateralization parameters
#[contracttype]
#[derive(Clone, Copy)]
pub struct CollateralParamsInput {
    /// The total amount of an asset the protocol accepts into the market.
    pub liq_cap: i128,
    /// Liquidation order
    pub pen_order: u32,
    pub util_cap: u32,
    /// Specifies what fraction of the underlying asset counts toward
    /// the portfolio collateral value [0%, 100%].
    pub discount: u32,
}
