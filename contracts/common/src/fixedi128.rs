use soroban_fixed_point_math::FixedPoint;

use crate::PERCENTAGE_FACTOR;

/// Fixed type with inner type of i128 and fixed denominator 10e9
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct FixedI128(i128);

impl FixedI128 {
    pub const DENOMINATOR: i128 = 1_000_000_000;
    pub const ZERO: FixedI128 = FixedI128(0);
    pub const ONE: FixedI128 = FixedI128(Self::DENOMINATOR);

    /// Returns inner value
    pub const fn into_inner(self) -> i128 {
        self.0
    }

    /// Construct FixedI128 from inner value
    pub fn from_inner<T: Into<i128>>(inner: T) -> FixedI128 {
        FixedI128(inner.into())
    }

    /// Construct fixed value from rational
    pub fn from_rational<N: Into<i128>, D: Into<i128>>(nom: N, denom: D) -> Option<FixedI128> {
        Self::DENOMINATOR
            .checked_mul(nom.into())?
            .checked_div(denom.into())
            .map(FixedI128)
    }

    /// Construct fixed value as percentage
    /// percentage expressed as 1% - 100, 100% - 10_000
    pub fn from_percentage<T: Into<i128>>(percentage: T) -> Option<FixedI128> {
        Self::from_rational(percentage, PERCENTAGE_FACTOR)
    }

    /// Construct fixed from int value
    pub fn from_int<T: Into<i128>>(value: T) -> Option<FixedI128> {
        FixedI128::DENOMINATOR
            .checked_mul(value.into())
            .map(FixedI128)
    }

    pub fn to_precision(self, precision: u32) -> Option<i128> {
        let prec_denom = 10i128.checked_pow(precision)?;

        self.0
            .checked_mul(prec_denom)?
            .checked_div(Self::DENOMINATOR)
    }

    /// Multiplication of two fixed values
    pub fn checked_mul(self, value: FixedI128) -> Option<FixedI128> {
        self.0
            .fixed_mul_floor(value.0, Self::DENOMINATOR)
            .map(FixedI128)
    }

    /// Division of two FixedI128 values
    pub fn checked_div(self, value: FixedI128) -> Option<FixedI128> {
        self.0
            .fixed_div_floor(value.0, Self::DENOMINATOR)
            .map(FixedI128)
    }

    /// Sum of two fixed values
    pub fn checked_add(self, value: FixedI128) -> Option<FixedI128> {
        self.0.checked_add(value.0).map(FixedI128)
    }

    /// Subtraction of two fixed values
    pub fn checked_sub(self, other: FixedI128) -> Option<FixedI128> {
        self.0.checked_sub(other.0).map(FixedI128)
    }

    /// Calculates product of fixed value and int value.
    /// Result is int value
    pub fn mul_int<T: Into<i128>>(self, other: T) -> Option<i128> {
        self.0
            .checked_mul(other.into())?
            .checked_div(Self::DENOMINATOR)
    }

    /// Calculates division of non fixed int value and fixed value, e.g.  other / self.
    /// Result is int value
    pub fn recip_mul_int<T: Into<i128>>(self, other: T) -> Option<i128> {
        Self::DENOMINATOR
            .checked_mul(other.into())?
            .checked_div(self.0)
    }

    /// Calculates division of non fixed int value and fixed value, e.g.  other / self and rounds towards infinity.
    /// Result is int value
    pub fn recip_mul_int_ceil<T: Into<i128>>(self, other: T) -> Option<i128> {
        let other = other.into();
        if other == 0 {
            return Some(0);
        }
        let mb_res = Self::DENOMINATOR.checked_mul(other)?.checked_div(self.0);
        mb_res.map(|res| {
            if res == 0 {
                1
            } else if other >= self.0 && other % self.0 == 0 {
                res
            } else {
                res + 1
            }
        })
    }

    /// Multiply inner value of fixed
    pub fn mul_inner<T: Into<i128>>(self, value: T) -> Option<FixedI128> {
        self.0.checked_mul(value.into()).map(FixedI128)
    }

    /// Div inner value of fixed
    pub fn div_inner<T: Into<i128>>(self, value: T) -> Option<FixedI128> {
        self.0.checked_div(value.into()).map(FixedI128)
    }

    /// Returns true if self is negative, false - when positive or zero
    pub fn is_negative(self) -> bool {
        self.0.is_negative()
    }

    /// Returns true if self is positive, false - when negative or zero
    pub fn is_positive(self) -> bool {
        self.0.is_positive()
    }

    /// Returns true if self is zero
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Returns max value
    pub fn max(self, other: FixedI128) -> FixedI128 {
        if self.0.gt(&other.0) {
            self
        } else {
            other
        }
    }

    /// Returns min value
    pub fn min(self, other: FixedI128) -> FixedI128 {
        if self.0.lt(&other.0) {
            self
        } else {
            other
        }
    }

    /// Returns absolute value
    pub fn abs(mut self) -> FixedI128 {
        self.0 = self.0.abs();
        self
    }
}
