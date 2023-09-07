use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env};

use crate::methods::utils::get_collat_coeff::get_collat_coeff;
use crate::storage::{read_reserve, read_token_total_supply};

pub fn collat_coeff(env: &Env, asset: &Address) -> Result<i128, Error> {
    let reserve = read_reserve(env, asset)?;
    let s_token_supply = read_token_total_supply(env, &reserve.s_token_address);
    let debt_token_supply = read_token_total_supply(env, &reserve.debt_token_address);

    get_collat_coeff(env, &reserve, s_token_supply, debt_token_supply)
        .map(|fixed| fixed.into_inner())
}
