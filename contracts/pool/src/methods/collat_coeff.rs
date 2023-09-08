use debt_token_interface::DebtTokenClient;
use pool_interface::types::error::Error;
use s_token_interface::STokenClient;
use soroban_sdk::{Address, Env};

use crate::storage::read_reserve;

use super::utils::get_collat_coeff::get_collat_coeff;

pub fn collat_coeff(env: &Env, asset: &Address) -> Result<i128, Error> {
    let reserve = read_reserve(env, asset)?;
    let s_token_supply = STokenClient::new(env, &reserve.s_token_address).total_supply();
    let debt_token_supply = DebtTokenClient::new(env, &reserve.debt_token_address).total_supply();

    get_collat_coeff(env, &reserve, s_token_supply, debt_token_supply)
        .map(|fixed| fixed.into_inner())
}
