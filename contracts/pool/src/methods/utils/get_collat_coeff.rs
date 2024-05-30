use common::FixedI128;
use pool_interface::types::{error::Error, reserve_data::ReserveData};
use soroban_sdk::Env;

use super::rate::get_actual_lender_accrued_rate;

/// Returns collateral coefficient
/// [underlying_balance + lender_ar * total_debt_token] / total_stoken
pub fn get_collat_coeff(
    env: &Env,
    reserve: &ReserveData,
    s_token_supply: i128,
    s_token_underlying_balance: i128,
    debt_token_supply: i128,
) -> Result<FixedI128, Error> {
    if s_token_supply == 0 {
        return Ok(FixedI128::ONE);
    }

    let collat_ar = get_actual_lender_accrued_rate(env, reserve)?;

    FixedI128::from_rational(
        s_token_underlying_balance
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

/// Returns compounded amount
/// [(s_token_underlying_balance + lender_ar * debt_token_supply) * amount] / s_token_supply
pub fn get_compounded_amount(
    env: &Env,
    reserve: &ReserveData,
    s_token_supply: i128,
    s_token_underlying_balance: i128,
    debt_token_supply: i128,
    amount: i128,
) -> Result<i128, Error> {
    if s_token_supply == 0 {
        return Ok(amount);
    }

    let collat_ar = get_actual_lender_accrued_rate(env, reserve)?;

    let x1 = collat_ar
        .mul_int(debt_token_supply)
        .ok_or(Error::CollateralCoeffMathError)?;

    let x2 = s_token_underlying_balance
        .checked_add(x1)
        .ok_or(Error::CollateralCoeffMathError)?;

    x2.checked_mul(amount)
        .ok_or(Error::CollateralCoeffMathError)?
        .checked_div(s_token_supply)
        .ok_or(Error::CollateralCoeffMathError)
}

/// Returns lp amount
/// s_token_supply * amount / (s_token_underlying_balance + lender_ar * debt_token_supply)
pub fn get_lp_amount(
    env: &Env,
    reserve: &ReserveData,
    s_token_supply: i128,
    s_token_underlying_balance: i128,
    debt_token_supply: i128,
    amount: i128,
) -> Result<i128, Error> {
    if s_token_supply == 0 {
        return Ok(amount);
    }

    let collat_ar = get_actual_lender_accrued_rate(env, reserve)?;

    let x1 = collat_ar
        .mul_int(debt_token_supply)
        .ok_or(Error::CollateralCoeffMathError)?;

    let nom = s_token_supply
        .checked_mul(amount)
        .ok_or(Error::CollateralCoeffMathError)?;

    let denom = s_token_underlying_balance
        .checked_add(x1)
        .ok_or(Error::CollateralCoeffMathError)?;

    nom.checked_div(denom)
        .ok_or(Error::CollateralCoeffMathError)
}
