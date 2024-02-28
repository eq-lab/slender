use common::{FixedI128, ALPHA_DENOMINATOR};
use pool_interface::types::error::Error;
use pool_interface::types::ir_params::IRParams;
use pool_interface::types::reserve_data::ReserveData;
use soroban_sdk::Env;

use super::get_elapsed_time::get_elapsed_time;

/// Calculate interest rate IR = MIN [ max_rate, base_rate / (1 - U)^alpha]
/// where
/// U - utilization, U = total_debt / total_collateral
/// ir_params.alpha - parameter, by default 1.43 expressed as 143 with denominator 100
/// ir_params.max_rate - maximal value of interest rate, by default 500% expressed as 50000 with denominator 10000
/// ir_params.initial_rate - base interest rate, by default 2%, expressed as 200 with denominator 10000
///
/// For (1-U)^alpha calculation use binomial approximation with four terms
/// (1-U)^a = 1 - alpha * U + alpha/2 * (alpha - 1) * U^2 - alpha/6 * (alpha-1) * (alpha-2) * U^3 + alpha/24 * (alpha-1) *(alpha-2) * (alpha-3) * U^4
pub fn calc_interest_rate(
    total_collateral: i128,
    total_debt: i128,
    ir_params: &IRParams,
) -> Option<FixedI128> {
    if total_collateral.is_negative() || total_debt.is_negative() {
        return None;
    }

    let u = FixedI128::from_rational(total_debt, total_collateral)?;

    if u.is_zero() {
        return Some(FixedI128::ZERO);
    }

    let max_rate = FixedI128::from_percentage(ir_params.max_rate)?;

    if u >= FixedI128::ONE {
        return Some(max_rate); // utilization shouldn't be greater or equal one
    }

    let alpha = FixedI128::from_rational(ir_params.alpha, ALPHA_DENOMINATOR)?;

    let neg_u = u.mul_inner(-1)?;
    let first_term = alpha.checked_mul(neg_u)?;

    let num_of_iterations = if u > FixedI128::from_rational(1, 2)? {
        19
    } else {
        3
    };
    let mut prev_term = first_term;
    let mut terms_sum = first_term;
    let mut alpha_mul = alpha;
    for i in 2..(num_of_iterations + 2) {
        alpha_mul = alpha_mul.checked_sub(FixedI128::ONE)?;
        let next_term = prev_term
            .checked_mul(neg_u)?
            .checked_mul(alpha_mul)?
            .div_inner(i)?;
        terms_sum = terms_sum.checked_add(next_term)?;
        prev_term = next_term;
    }

    let denom = FixedI128::ONE.checked_add(terms_sum)?;

    if denom.is_negative() {
        return Some(max_rate);
    }

    let initial_rate = FixedI128::from_percentage(ir_params.initial_rate)?;

    let ir = initial_rate.checked_div(denom)?;

    Some(FixedI128::min(ir, max_rate))
}

/// Calculate accrued rate on time `t` AR(t) = AR(t-1)*(1 + r(t-1)*elapsed_time)
/// where:
///     AR(t-1) - prev value of accrued rate
///     r(t-1) - prev value of interest rate
///     elapsed_time - elapsed time in seconds from last accrued rate update
pub fn calc_next_accrued_rate(
    prev_ar: FixedI128,
    ir: FixedI128,
    elapsed_time: u64,
) -> Option<FixedI128> {
    let delta_time = FixedI128::from_rational(elapsed_time, common::ONE_YEAR)?;
    prev_ar.checked_mul(FixedI128::ONE.checked_add(ir.checked_mul(delta_time)?)?)
}

#[derive(Debug, Clone, Copy)]
pub struct AccruedRates {
    pub lender_ar: FixedI128,
    pub borrower_ar: FixedI128,
    pub lender_ir: FixedI128,
    pub borrower_ir: FixedI128,
}

/// Calculates lender and borrower accrued/interest rates
pub fn calc_accrued_rates(
    total_collateral: i128,
    total_debt: i128,
    elapsed_time: u64,
    ir_params: IRParams,
    reserve_data: &ReserveData,
) -> Option<AccruedRates> {
    let borrower_ir = calc_interest_rate(total_collateral, total_debt, &ir_params)?;

    let scale_coeff = FixedI128::from_percentage(ir_params.scaling_coeff)?;
    let lender_ir = borrower_ir.checked_mul(scale_coeff)?;

    let borrower_ar = calc_next_accrued_rate(
        FixedI128::from_inner(reserve_data.borrower_ar),
        borrower_ir,
        elapsed_time,
    )?;

    let lender_ar = calc_next_accrued_rate(
        FixedI128::from_inner(reserve_data.lender_ar),
        lender_ir,
        elapsed_time,
    )?;

    Some(AccruedRates {
        lender_ar,
        borrower_ar,
        lender_ir,
        borrower_ir,
    })
}

/// Returns lender accrued rate corrected for the current time
pub fn get_actual_lender_accrued_rate(
    env: &Env,
    reserve: &ReserveData,
) -> Result<FixedI128, Error> {
    let (_, elapsed_time) = get_elapsed_time(env, reserve.last_update_timestamp);
    let prev_ar = FixedI128::from_inner(reserve.lender_ar);

    if elapsed_time == 0 {
        Ok(prev_ar)
    } else {
        let lender_ir = FixedI128::from_inner(reserve.lender_ir);
        calc_next_accrued_rate(prev_ar, lender_ir, elapsed_time)
            .ok_or(Error::CollateralCoeffMathError)
    }
}

/// Returns borrower accrued rate corrected for the current time
pub fn get_actual_borrower_accrued_rate(
    env: &Env,
    reserve: &ReserveData,
) -> Result<FixedI128, Error> {
    let (_, elapsed_time) = get_elapsed_time(env, reserve.last_update_timestamp);
    let prev_ar = FixedI128::from_inner(reserve.borrower_ar);

    if elapsed_time == 0 {
        Ok(prev_ar)
    } else {
        let debt_ir = FixedI128::from_inner(reserve.borrower_ir);
        calc_next_accrued_rate(prev_ar, debt_ir, elapsed_time).ok_or(Error::DebtCoeffMathError)
    }
}
