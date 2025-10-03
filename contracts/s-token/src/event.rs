use soroban_sdk::{contractevent, Address, String};

#[contractevent(topics=["approve"], data_format = "map")]
pub(crate) struct ApproveEvent {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
    pub expiration_ledger: u32,
}

#[contractevent(topics=["transfer"], data_format = "single-value")]
pub(crate) struct TransferEvent {
    #[topic]
    pub from: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
}

#[contractevent(topics=["mint"], data_format = "single-value")]
pub(crate) struct MintEvent {
    #[topic]
    pub admin: Address,
    #[topic]
    pub to: Address,
    pub amount: i128,
}

#[contractevent(topics=["clawback"], data_format = "single-value")]
pub(crate) struct ClawbackEvent {
    #[topic]
    pub from: Address,
    pub amount: i128,
}

#[contractevent(topics=["set_authorized"], data_format = "single-value")]
pub(crate) struct SetAuthorizedEvent {
    #[topic]
    pub id: Address,
    pub authorize: bool,
}

#[contractevent(topics=["burn"], data_format = "single-value")]
pub(crate) struct BurnEvent {
    #[topic]
    pub from: Address,
    pub amount: i128,
}

#[contractevent(topics=["initialized"], data_format = "map")]
pub(crate) struct InitializedEvent {
    #[topic]
    pub underlying_asset: Address,
    #[topic]
    pub pool: Address,
    pub decimals: u32,
    pub name: String,
    pub symbol: String,
}
