use common::FixedI128;
use debt_token_interface::DebtTokenClient;
use pool_interface::types::{error::Error, reserve_data::ReserveData};
use s_token_interface::STokenClient;
use soroban_sdk::{Address, Env};

use crate::storage::{read_reserve, read_stoken_underlying_balance};

use super::rate::get_actual_lender_accrued_rate;

pub fn collat_coeff(env: &Env, asset: &Address) -> Result<i128, Error> {
    let reserve = read_reserve(env, asset)?;
    #[cfg(not(feature = "exceeded-limit-fix"))]
    let s_token_supply = STokenClient::new(env, &reserve.s_token_address).total_supply();
    #[cfg(not(feature = "exceeded-limit-fix"))]
    let debt_token_supply = DebtTokenClient::new(env, &reserve.debt_token_address).total_supply();
    #[cfg(feature = "exceeded-limit-fix")]
    let s_token_supply = read_token_total_supply(env, &reserve.s_token_address);
    #[cfg(feature = "exceeded-limit-fix")]
    let debt_token_supply = read_token_total_supply(env, &reserve.debt_token_address);

    get_collat_coeff(env, &reserve, s_token_supply, debt_token_supply)
        .map(|fixed| fixed.into_inner())
}

/// Returns collateral coefficient
/// collateral_coeff = [underlying_balance + lender_ar * total_debt_token]/total_stoken
pub fn get_collat_coeff(
    env: &Env,
    reserve: &ReserveData,
    s_token_supply: i128,
    debt_token_supply: i128,
) -> Result<FixedI128, Error> {
    if s_token_supply == 0 {
        return Ok(FixedI128::ONE);
    }

    let collat_ar = get_actual_lender_accrued_rate(env, reserve)?;
    let balance = read_stoken_underlying_balance(env, &reserve.s_token_address);

    FixedI128::from_rational(
        balance
            .checked_add(
                collat_ar
                    .mul_int(debt_token_supply)
                    .ok_or(Error::CollateralCoeffMathError)?,
            )
            .ok_or(Error::CollateralCoeffMathError)?,
        s_token_supply,
    )
    .ok_or(Error::CollateralCoeffMathError)
}
