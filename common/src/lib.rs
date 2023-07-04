#![deny(warnings)]
#![no_std]

use fixed_point_math::FixedPoint;

pub const RATE_DENOMINATOR: i128 = 1_000_000_000;

pub trait RateMath<T: Into<i128>> {
    /// result = self * rate / RATE_DENOMINATOR
    fn mul_rate_floor(self, rate: T) -> Option<i128>;

    /// result = self * rate / RATE_DENOMINATOR
    fn mul_rate_ceil(self, rate: T) -> Option<i128>;

    /// result = self * RATE_DENOMINATOR / rate
    fn div_rate_floor(self, rate: T) -> Option<i128>;

    /// result = self * RATE_DENOMINATOR / rate
    fn div_rate_ceil(self, rate: T) -> Option<i128>;
}

impl<T: Into<i128>> RateMath<T> for T {
    fn mul_rate_floor(self, rate: T) -> Option<i128> {
        Into::<i128>::into(self).fixed_mul_floor(rate.into(), RATE_DENOMINATOR)
    }

    fn mul_rate_ceil(self, rate: T) -> Option<i128> {
        Into::<i128>::into(self).fixed_mul_ceil(rate.into(), RATE_DENOMINATOR)
    }

    fn div_rate_floor(self, rate: T) -> Option<i128> {
        Into::<i128>::into(self).fixed_div_floor(rate.into(), RATE_DENOMINATOR)
    }

    fn div_rate_ceil(self, rate: T) -> Option<i128> {
        Into::<i128>::into(self).fixed_div_ceil(rate.into(), RATE_DENOMINATOR)
    }
}
