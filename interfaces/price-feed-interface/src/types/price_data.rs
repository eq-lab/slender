use soroban_sdk::contracttype;

/// Price data for an asset at a specific timestamp
#[contracttype]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}
