use fixed_point_math::FixedPoint;

/// Fixed type with inner type of i128 and fixed denominator 10e9
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct FixedI128(i128);

impl FixedI128 {
    pub const DENOMINATOR: i128 = 1_000_000_000;

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

    /// Construct fixed from int value
    pub fn from_value<T: Into<i128>>(value: T) -> Option<FixedI128> {
        FixedI128::DENOMINATOR
            .checked_mul(value.into())
            .map(FixedI128)
    }

    /// Multiplication of two fixed values
    pub fn mul(self, value: FixedI128) -> Option<FixedI128> {
        self.0
            .fixed_mul_floor(value.0, Self::DENOMINATOR)
            .map(FixedI128)
    }

    /// Division of two FixedI128 values
    pub fn div(self, value: FixedI128) -> Option<FixedI128> {
        self.0
            .fixed_div_floor(value.0, Self::DENOMINATOR)
            .map(FixedI128)
    }

    /// Sum of two fixed values
    pub fn add(self, value: FixedI128) -> Option<FixedI128> {
        self.0.checked_add(value.0).map(FixedI128)
    }

    /// Subtraction of two fixed values
    pub fn sub(self, other: FixedI128) -> Option<FixedI128> {
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

    /// Multiply inner value of fixed
    pub fn mul_inner<T: Into<i128>>(self, value: T) -> Option<FixedI128> {
        self.0.checked_mul(value.into()).map(FixedI128)
    }

    /// Div inner value of fixed
    pub fn div_inner<T: Into<i128>>(self, value: T) -> Option<FixedI128> {
        self.0.checked_div(value.into()).map(FixedI128)
    }
}
