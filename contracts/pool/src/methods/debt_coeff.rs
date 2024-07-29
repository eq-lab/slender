use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env};

use crate::{read_pool_config, storage::read_reserve};

use super::utils::rate::get_actual_borrower_accrued_rate;

pub fn debt_coeff(env: &Env, asset: &Address) -> Result<i128, Error> {
    get_actual_borrower_accrued_rate(env, &read_reserve(env, asset)?, &read_pool_config(env)?)
        .map(|fixed| fixed.into_inner())
}
