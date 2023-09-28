//! Note, the PriceFeedTrait, and the PriceData are SEP-40 compatible.
//! More details can be found at the following link:
//! https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0040.md

#![deny(warnings)]
#![no_std]

use soroban_sdk::{contractclient, contractspecfn, contracttype, Address, Env, Vec};

pub struct Spec;

/// Price data for an asset at a specific timestamp
#[contracttype]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
}

/// Oracle feed interface description
#[contractspecfn(name = "Spec", export = false)]
#[contractclient(name = "PriceFeedClient")]
pub trait PriceFeedTrait {
    /// Return the base asset the price is reported in
    fn base(env: Env) -> Address;

    /// Return all assets quoted by the price feed
    fn assets(env: Env) -> Vec<Address>;

    /// Return the number of decimals for all assets quoted by the oracle
    fn decimals(env: Env) -> u32;

    /// Return default tick period timeframe (in seconds)
    fn resolution(env: Env) -> u32;

    /// Get price in base asset at specific timestamp
    fn price(env: Env, asset: Address, timestamp: u64) -> Option<PriceData>;

    /// Get last N price records
    fn prices(env: Env, asset: Address, records: u32) -> Option<Vec<PriceData>>;

    /// Get the most recent price for an asset
    fn lastprice(env: Env, asset: Address) -> Option<PriceData>;

    /// Sets price in base asset for a given asset. Note: not a SEP-40 method.
    fn init(env: Env, asset: Address, price: i128);
}
