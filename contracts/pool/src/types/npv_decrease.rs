pub struct NpvDecrease {
    pub borrow_amount_in_base: i128,
    pub withdraw_amount_in_base_discounted: i128,
}

impl NpvDecrease {
    pub fn zero() -> Self {
        Self {
            borrow_amount_in_base: 0,
            withdraw_amount_in_base_discounted: 0,
        }
    }
}
