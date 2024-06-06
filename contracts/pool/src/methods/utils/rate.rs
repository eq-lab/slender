use common::{FixedI128, ALPHA_DENOMINATOR};
use pool_interface::types::error::Error;
use pool_interface::types::pool_config::PoolConfig;
use pool_interface::types::reserve_data::ReserveData;
use soroban_sdk::Env;

use super::get_elapsed_time::get_elapsed_time;

pub fn calc_interest_rate(
    total_collateral: i128,
    total_debt: i128,
    pool_config: &PoolConfig,
) -> Option<FixedI128> {
    if total_collateral.is_negative() || total_debt.is_negative() {
        return None;
    }

    let u = FixedI128::from_rational(total_debt, total_collateral)?;

    if u.is_zero() {
        return Some(FixedI128::ZERO);
    }

    let max_rate = FixedI128::from_percentage(pool_config.ir_max_rate)?;

    if u >= FixedI128::ONE {
        return Some(max_rate);
    }

    let alpha = FixedI128::from_rational(pool_config.ir_alpha, ALPHA_DENOMINATOR)?;

    let neg_u = u.mul_inner(-1)?;
    let first_term = alpha.checked_mul(neg_u)?;

    let num_of_iterations = if u > FixedI128::from_rational(1, 2)? {
        20
    } else {
        4
    };
    let mut prev_term = first_term;
    let mut terms_sum = first_term;
    let mut alpha_mul = alpha;
    for i in 2..=num_of_iterations {
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

    let initial_rate = FixedI128::from_percentage(pool_config.ir_initial_rate)?;

    let ir = initial_rate.checked_div(denom)?;

    Some(FixedI128::min(ir, max_rate))
}

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

pub fn calc_accrued_rates(
    total_collateral: i128,
    total_debt: i128,
    elapsed_time: u64,
    pool_config: &PoolConfig,
    reserve_data: &ReserveData,
) -> Option<AccruedRates> {
    let borrower_ir = calc_interest_rate(total_collateral, total_debt, pool_config)?;

    let scale_coeff = FixedI128::from_percentage(pool_config.ir_scaling_coeff)?;
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

pub fn get_actual_lender_accrued_rate(
    env: &Env,
    reserve: &ReserveData,
    pool_config: &PoolConfig,
) -> Result<FixedI128, Error> {
    let (_, elapsed_time) = get_elapsed_time(
        env,
        reserve.last_update_timestamp,
        pool_config.timestamp_window,
    );
    let prev_ar = FixedI128::from_inner(reserve.lender_ar);

    if elapsed_time == 0 {
        Ok(prev_ar)
    } else {
        let lender_ir = FixedI128::from_inner(reserve.lender_ir);
        calc_next_accrued_rate(prev_ar, lender_ir, elapsed_time)
            .ok_or(Error::CollateralCoeffMathError)
    }
}

pub fn get_actual_borrower_accrued_rate(
    env: &Env,
    reserve: &ReserveData,
    pool_config: &PoolConfig,
) -> Result<FixedI128, Error> {
    let (_, elapsed_time) = get_elapsed_time(
        env,
        reserve.last_update_timestamp,
        pool_config.timestamp_window,
    );
    let prev_ar = FixedI128::from_inner(reserve.borrower_ar);

    if elapsed_time == 0 {
        Ok(prev_ar)
    } else {
        let debt_ir = FixedI128::from_inner(reserve.borrower_ir);
        calc_next_accrued_rate(prev_ar, debt_ir, elapsed_time).ok_or(Error::DebtCoeffMathError)
    }
}
