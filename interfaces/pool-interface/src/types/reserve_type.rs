use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReserveType {
    /// Fungible reserve for which created sToken and debtToken
    Fungible(Address, Address),
    /// RWA reserve
    RWA,
}
