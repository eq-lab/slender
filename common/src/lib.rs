#![deny(warnings)]
#![no_std]

mod fixedi128;
pub mod percentage_math;
pub mod rate_math;
#[cfg(test)]
mod test;

pub use fixed_point_math::FixedPoint;
pub use fixedi128::*;

/// Denominator for alpha, used in interest rate calculation
pub const ALPHA_DENOMINATOR: u32 = 100;
