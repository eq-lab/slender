use common::{FixedI128, ALPHA_DENOMINATOR};
use pool_interface::{Error, IRParams, ReserveData};
use s_token_interface::STokenClient;
use soroban_sdk::{Address, Env};

use crate::storage::write_reserve;

/// Calculate interest rate IR = MIN [ max_rate, base_rate / (1 - U)^alpha]
/// where
/// U - utilization, U = total_debt / total_collateral
/// ir_params.alpha - parameter, by default 1.43 expressed as 143 with denominator 100
/// ir_params.max_rate - maximal value of interest rate, by default 500% expressed as 50000 with denominator 10000
/// ir_params.initial_rate - base interest rate, by default 2%, expressed as 200 with denominator 10000
///
/// For (1-U)^alpha calculation use binomial approximation with four terms
/// (1-U)^a = 1 - alpha * U + alpha/2 * (alpha - 1) * U^2 - alpha/6 * (alpha-1) * (alpha-2) * U^3 + alpha/24 * (alpha-1) *(alpha-2) * (alpha-3) * U^4
#[allow(dead_code)]
pub fn calc_interest_rate(
    total_collateral: i128,
    total_debt: i128,
    ir_params: &IRParams,
) -> Option<FixedI128> {
    if total_collateral.is_negative() || total_debt.is_negative() {
        return None;
    }

    let u = FixedI128::from_rational(total_debt, total_collateral)?;
    let max_rate = FixedI128::from_percentage(ir_params.max_rate)?;

    if u >= FixedI128::ONE {
        return Some(max_rate); // utilization shouldn't be greater or equal one
    }

    let alpha = FixedI128::from_rational(ir_params.alpha, ALPHA_DENOMINATOR)?;

    let alpha_minus_one = alpha.checked_sub(FixedI128::ONE)?;
    let alpha_minus_two = alpha_minus_one.checked_sub(FixedI128::ONE)?;
    let alpha_minus_three = alpha_minus_two.checked_sub(FixedI128::ONE)?;

    let first_term = alpha.checked_mul(u)?;
    let second_term = first_term
        .checked_mul(u)?
        .checked_mul(alpha_minus_one)?
        .div_inner(2)?;
    let third_term = second_term
        .checked_mul(u)?
        .checked_mul(alpha_minus_two)?
        .div_inner(3)?;
    let fourth_term = third_term
        .checked_mul(u)?
        .checked_mul(alpha_minus_three)?
        .div_inner(4)?;

    let denom = FixedI128::ONE
        .checked_sub(first_term)?
        .checked_add(second_term)?
        .checked_sub(third_term)?
        .checked_add(fourth_term)?;

    if denom.is_negative() {
        return Some(max_rate);
    }

    let initial_rate = FixedI128::from_percentage(ir_params.initial_rate)?;

    let ir = initial_rate.checked_div(denom)?;

    Some(FixedI128::min(ir, max_rate))
}

/// Calculate accrued rate coefficient AR(t) = AR(t-1)*(1 + r(t-1)*elapsed_time)
/// where:
///     AR(t-1) - prev value of accrued rate
///     r(t-1) - prev value of interest rate
///     elapsed_time - elapsed time from last accrued rate update
pub fn calc_accrued_rate_coeff(
    prev_ar: FixedI128,
    ir: FixedI128,
    elapsed_time: u64,
) -> Option<FixedI128> {
    let delta_time = FixedI128::from_rational(elapsed_time, common::ONE_YEAR)?;
    prev_ar.checked_mul(FixedI128::ONE.checked_add(ir.checked_mul(delta_time)?)?)
}

/// Calculates collateral and debt accrued coefficients and updates reserve data
pub fn update_accrued_rates(
    env: &Env,
    asset: Address,
    reserve_data: ReserveData,
) -> Result<ReserveData, Error> {
    let current_time = env.ledger().timestamp();
    let elapsed_time = current_time
        .checked_sub(reserve_data.last_update_timestamp)
        .ok_or(Error::AccruedRateMathError)?;

    if elapsed_time == 0 {
        return Ok(reserve_data);
    }

    let s_token = STokenClient::new(env, &reserve_data.s_token_address);
    let total_collateral = s_token.total_supply();

    let debt_token = STokenClient::new(env, &reserve_data.debt_token_address);
    let total_debt = debt_token.total_supply();

    let debt_ir = calc_interest_rate(total_collateral, total_debt, &reserve_data.ir_params)
        .ok_or(Error::AccruedRateMathError)?;

    let scale_coeff = FixedI128::from_percentage(reserve_data.ir_params.scaling_coeff)
        .ok_or(Error::AccruedRateMathError)?;
    let lend_ir = debt_ir
        .checked_mul(scale_coeff)
        .ok_or(Error::AccruedRateMathError)?;

    let debt_accrued_rate = calc_accrued_rate_coeff(
        FixedI128::from_inner(reserve_data.debt_accrued_rate),
        debt_ir,
        elapsed_time,
    )
    .ok_or(Error::AccruedRateMathError)?
    .into_inner();

    let collat_accrued_rate = calc_accrued_rate_coeff(
        FixedI128::from_inner(reserve_data.collat_accrued_rate),
        lend_ir,
        elapsed_time,
    )
    .ok_or(Error::AccruedRateMathError)?
    .into_inner();

    let mut reserve_data = reserve_data;
    reserve_data.collat_accrued_rate = collat_accrued_rate;
    reserve_data.debt_accrued_rate = debt_accrued_rate;
    reserve_data.last_update_timestamp = current_time;

    write_reserve(env, asset, &reserve_data);

    Ok(reserve_data)
}
