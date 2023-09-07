use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env};

use crate::storage::read_reserve;

use super::utils::rate::get_actual_borrower_accrued_rate;

pub fn debt_coeff(env: &Env, asset: &Address) -> Result<i128, Error> {
    let reserve = read_reserve(env, asset)?;
    get_actual_borrower_accrued_rate(env, &reserve).map(|fixed| fixed.into_inner())
}
