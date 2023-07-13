use common::{FixedI128, ALPHA_DENOMINATOR};

/// Calculate interest rate IR = MIN [ max_rate, base_rate / (1 - U)^alpha]
/// where
/// U - utilization, U = total_debt / total_collateral
/// alpha - parameter, by default 1.43 expressed as 143 with denominator 100
/// max_rate - maximal value of interest rate, by default 500% expressed as 50000 with denominator 10000
/// base_rate - base interest rate, by default 2%, expressed as 200 with denominator 10000
///
/// For (1-U)^alpha calculation use binomial approximation with four terms
/// (1-U)^a = 1 - alpha * U + alpha/2 * (alpha - 1) * U^2 - alpha/6 * (alpha-1) * (alpha-2) * U^3 + alpha/24 * (alpha-1) *(alpha-2) * (alpha-3) * U^4
#[allow(dead_code)]
pub fn calc_interest_rate(
    total_collateral: i128,
    total_debt: i128,
    alpha: u32,     // 143 / 100
    max_rate: u32,  // 50000 / 10000
    base_rate: u32, // 200 / 10000
) -> Option<FixedI128> {
    if total_collateral.is_negative() || total_debt.is_negative() {
        return None;
    }

    let u = FixedI128::from_rational(total_debt, total_collateral)?;
    let max_rate = FixedI128::from_percentage(max_rate)?;

    if u >= FixedI128::ONE {
        return Some(max_rate); // utilization shouldn't be greater or equal one
    }

    let alpha = FixedI128::from_rational(alpha, ALPHA_DENOMINATOR)?;

    let alpha_minus_one = alpha.sub(FixedI128::ONE)?;
    let alpha_minus_two = alpha_minus_one.sub(FixedI128::ONE)?;
    let alpha_minus_three = alpha_minus_two.sub(FixedI128::ONE)?;

    let first_term = alpha.mul(u)?;
    let second_term = first_term.mul(u)?.mul(alpha_minus_one)?.div_inner(2)?;
    let third_term = second_term.mul(u)?.mul(alpha_minus_two)?.div_inner(3)?;
    let fourth_term = third_term.mul(u)?.mul(alpha_minus_three)?.div_inner(4)?;

    let denom = FixedI128::ONE
        .sub(first_term)?
        .add(second_term)?
        .sub(third_term)?
        .add(fourth_term)?;

    if denom.is_negative() {
        return Some(max_rate);
    }

    let base_rate_fixed = FixedI128::from_percentage(base_rate)?;

    let ir = base_rate_fixed.div(denom)?;

    Some(FixedI128::min(ir, max_rate))
}
