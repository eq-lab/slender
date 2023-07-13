#![deny(warnings)]
#![no_std]

pub mod percentage_math;
pub mod rate_math;

mod fixedi128;

pub use fixed_point_math::FixedPoint;
pub use fixedi128::*;

/// Denominator for alpha, used in interest rate calculation
pub const ALPHA_DENOMINATOR: u32 = 100;
