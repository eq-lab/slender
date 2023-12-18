use common::FixedI128;
use pool_interface::types::{error::Error, reserve_data::ReserveData};
use soroban_sdk::Env;

use crate::storage::read_stoken_underlying_balance;

use super::rate::get_actual_lender_accrued_rate;

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
