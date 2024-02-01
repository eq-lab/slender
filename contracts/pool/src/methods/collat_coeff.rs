use pool_interface::types::{error::Error, reserve_type::ReserveType};
use soroban_sdk::{Address, Env};

use crate::storage::{read_reserve, read_token_total_supply};

use super::utils::get_collat_coeff::get_collat_coeff;

pub fn collat_coeff(env: &Env, asset: &Address) -> Result<i128, Error> {
    let reserve = read_reserve(env, asset)?;

    if let ReserveType::Fungible(s_token_address, debt_token_address) = &reserve.reserve_type {
        get_collat_coeff(
            env,
            &reserve,
            s_token_address,
            read_token_total_supply(env, s_token_address),
            read_token_total_supply(env, debt_token_address),
        )
        .map(|fixed| fixed.into_inner())
    } else {
        return Err(Error::NotFungible);
    }
}
