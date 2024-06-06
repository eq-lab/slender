use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env};

use crate::{
    read_pool_config, read_token_balance,
    storage::{read_reserve, read_token_total_supply},
};

use super::utils::get_collat_coeff::get_collat_coeff;

pub fn collat_coeff(env: &Env, asset: &Address) -> Result<i128, Error> {
    let reserve = read_reserve(env, asset)?;

    let (s_token_address, debt_token_address) = reserve.get_fungible()?;
    let pool_config = read_pool_config(env)?;

    get_collat_coeff(
        env,
        &reserve,
        &pool_config,
        read_token_total_supply(env, s_token_address),
        read_token_balance(env, asset, s_token_address),
        read_token_total_supply(env, debt_token_address),
    )
    .map(|fixed| fixed.into_inner())
}
