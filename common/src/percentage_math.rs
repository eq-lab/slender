/// Percent representation
pub const PERCENTAGE_FACTOR: u32 = 10000;

const HALF_PERCENT: u32 = PERCENTAGE_FACTOR / 2;

pub trait PercentageMath<T: Into<i128>> {
    fn percent_mul(self, percentage: T) -> Option<i128>;

    fn percent_div(self, percentage: T) -> Option<i128>;
}

impl<T: Into<i128>, V: Into<i128>> PercentageMath<T> for V {
    fn percent_mul(self, percentage: T) -> Option<i128> {
        let self_i128: i128 = self.into();
        let percentage_i128: i128 = percentage.into();
        if self_i128 == 0 || percentage_i128 == 0 {
            return Some(0);
        }

        if self_i128 > (i128::MAX - (HALF_PERCENT as i128)) / percentage_i128 {
            return None;
        }

        Some((self_i128 * percentage_i128 + (HALF_PERCENT as i128)) / (PERCENTAGE_FACTOR as i128))
    }

    fn percent_div(self, percentage: T) -> Option<i128> {
        let percentage_i128: i128 = percentage.into();
        if percentage_i128 == 0 {
            return None;
        }

        let self_i128: i128 = self.into();
        let half_percentage = percentage_i128 / 2;

        if self_i128 > (i128::MAX - half_percentage) / (PERCENTAGE_FACTOR as i128) {
            return None;
        }

        Some((self_i128 * (PERCENTAGE_FACTOR as i128) + half_percentage) / percentage_i128)
    }
}
