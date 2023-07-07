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

/// Percent representation
pub const PERCENTAGE_FACTOR: i128 = 10000;

const HALF_PERCENT: i128 = PERCENTAGE_FACTOR / 2;

pub trait PercentageMath<T: Into<i128>> {
    fn percent_mul(self, percentage: T) -> Option<i128>;

    fn percent_div(self, percentage: T) -> Option<i128>;
}

impl<T: Into<i128>> PercentageMath<T> for T {
    fn percent_mul(self, percentage: T) -> Option<i128> {
        let self_i128 = Into::<i128>::into(self);
        let percentage_i128 = Into::<i128>::into(percentage);
        if self_i128 == 0 || percentage_i128 == 0 {
            return Some(0);
        }

        if self_i128 > (i128::MAX - HALF_PERCENT) / PERCENTAGE_FACTOR {
            return None;
        }

        Some((self_i128 * percentage_i128 + HALF_PERCENT) / PERCENTAGE_FACTOR)
    }

    fn percent_div(self, percentage: T) -> Option<i128> {
        let percentage_i128 = Into::<i128>::into(percentage);
        if percentage_i128 == 0 {
            return None;
        }

        let self_i128 = Into::<i128>::into(self);

        let half_percentage = percentage_i128 / 2;

        if self_i128 > (i128::MAX - half_percentage) / PERCENTAGE_FACTOR {
            return None;
        }

        Some((self_i128 * PERCENTAGE_FACTOR + half_percentage) / percentage_i128)
    }
}
