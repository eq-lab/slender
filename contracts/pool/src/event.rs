use soroban_sdk::{contractevent, Address};

#[contractevent(topics=["initialize"], data_format = "map")]
pub(crate) struct InitializedEvent {
    #[topic]
    pub admin: Address,
    #[topic]
    pub base_asset_address: Address,
    pub ir_alpha: u32,
    pub ir_initial_rate: u32,
    pub ir_max_rate: u32,
    pub ir_scaling_coeff: u32,
    pub base_asset_decimals: u32,
    pub initial_health: u32,
    pub grace_period: u64,
    pub timestamp_window: u64,
    pub flash_loan_fee: u32,
    pub user_assets_limit: u32,
    pub min_collat_amount: i128,
    pub min_debt_amount: i128,
    pub liquidation_protocol_fee: u32,
}

#[contractevent(topics=["reserve_used_as_coll_enabled"], data_format = "single-value")]
pub(crate) struct ReserveUsedAsCollEnabledEvent {
    #[topic]
    pub who: Address,
    pub asset: Address,
}

#[contractevent(topics=["reserve_used_as_coll_disabled"], data_format = "single-value")]
pub(crate) struct ReserveUsedAsCollDisabledEvent {
    #[topic]
    pub who: Address,
    pub asset: Address,
}

#[contractevent(topics=["deposit"], data_format = "map")]
pub(crate) struct DepositEvent {
    #[topic]
    pub who: Address,
    pub asset: Address,
    pub amount: i128,
}

#[contractevent(topics=["withdraw"], data_format = "map")]
pub(crate) struct WithdrawEvent {
    #[topic]
    pub who: Address,
    pub to: Address,
    pub asset: Address,
    pub amount: i128,
}

#[contractevent(topics=["borrow"], data_format = "map")]
pub(crate) struct BorrowEvent {
    #[topic]
    pub who: Address,
    pub asset: Address,
    pub amount: i128,
}

#[contractevent(topics=["repay"], data_format = "map")]
pub(crate) struct RepayEvent {
    #[topic]
    pub who: Address,
    pub asset: Address,
    pub amount: i128,
}

#[contractevent(topics=["collat_config_change"], data_format = "map")]
pub(crate) struct CollatConfigChangeEvent {
    #[topic]
    pub asset: Address,
    pub liq_cap: i128,
    pub pen_order: u32,
    pub util_cap: u32,
    pub discount: u32,
}

#[contractevent(topics=["borrowing_enabled"], data_format = "single-value")]
pub(crate) struct BorrowingEnabledEvent {
    #[topic]
    pub asset: Address,
}

#[contractevent(topics=["borrowing_disabled"], data_format = "single-value")]
pub(crate) struct BorrowingDisabledEvent {
    #[topic]
    pub asset: Address,
}

#[contractevent(topics=["reserve_status_changed"], data_format = "single-value")]
pub(crate) struct ReserveStatusChangedEvent {
    #[topic]
    pub asset: Address,
    pub activated: bool,
}

#[contractevent(topics=["liquidation"], data_format = "map")]
pub(crate) struct LiquidationEvent {
    #[topic]
    pub who: Address,
    pub covered_debt: i128,
    pub liquidated_collateral: i128,
}

#[contractevent(topics=["flash_loan"], data_format = "map")]
pub(crate) struct FlashLoanEvent {
    #[topic]
    pub who: Address,
    #[topic]
    pub receiver: Address,
    #[topic]
    pub asset: Address,
    pub amount: i128,
    pub premium: i128,
    pub borrow: bool,
}
