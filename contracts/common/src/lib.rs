#![deny(warnings)]
#![no_std]

mod fixedi128;
#[cfg(test)]
mod test;

pub use fixedi128::*;

/// Denominator for alpha, used in interest rate calculation
pub const ALPHA_DENOMINATOR: u32 = 100;

/// Percent representation
pub const PERCENTAGE_FACTOR: u32 = 10_000;

///Seconds in year. Equal 365.25 * 24 * 60 * 60
pub const ONE_YEAR: u64 = 31_557_600;
