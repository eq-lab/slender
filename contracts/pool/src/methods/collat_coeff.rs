use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env};

use crate::storage::{read_reserve, read_stoken_underlying_balance, read_token_total_supply};

use super::utils::{
    get_collat_coeff::get_collat_coeff, get_fungible_lp_tokens::get_fungible_lp_tokens,
};

pub fn collat_coeff(env: &Env, asset: &Address) -> Result<i128, Error> {
    let reserve = read_reserve(env, asset)?;

    let (s_token_address, debt_token_address) = get_fungible_lp_tokens(&reserve)?;

    get_collat_coeff(
        env,
        &reserve,
        read_token_total_supply(env, s_token_address),
        read_stoken_underlying_balance(env, s_token_address),
        read_token_total_supply(env, debt_token_address),
    )
    .map(|fixed| fixed.into_inner())
}
