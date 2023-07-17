use common::FixedI128;

use crate::rate::*;

fn get_default_ir_params() -> (u32, u32, u32) {
    let alpha = 143;
    let max_rate = 50000;
    let base_rate = 200;

    (alpha, max_rate, base_rate)
}

#[test]
fn calc_ir_utilization_is_zero() {
    let total_collateral = 1000;
    let total_debt = 0;
    let (alpha, max_rate, base_rate) = get_default_ir_params();

    let ir = calc_interest_rate(total_collateral, total_debt, alpha, max_rate, base_rate);

    assert_eq!(ir, FixedI128::from_percentage(base_rate));
}

#[test]
fn calc_ir_utilization_is_one() {
    let total_collateral = 1;
    let total_debt = 1;
    let (alpha, max_rate, base_rate) = get_default_ir_params();

    let ir = calc_interest_rate(total_collateral, total_debt, alpha, max_rate, base_rate);

    assert_eq!(ir, FixedI128::from_percentage(max_rate));
}

#[test]
fn calc_ir() {
    let (alpha, max_rate, base_rate) = get_default_ir_params();

    //utilization = 0.2, ir ~ 0.027517810, ir = 0.02/(1-0.2)^1.43 = 0,0275176482
    let total_debt = 20;
    let total_collateral: i128 = 100;
    let ir = calc_interest_rate(total_collateral, total_debt, alpha, max_rate, base_rate).unwrap();
    assert_eq!(
        ir,
        FixedI128::from_rational(27517810, 1_000_000_000).unwrap()
    );

    //utilization = 0.5, ir ~ 0.053966913, ir = 0.02/(1 - 0.5)^1.43 = 0,0538893431
    let total_debt = 50;
    let total_collateral: i128 = 100;
    let ir = calc_interest_rate(total_collateral, total_debt, alpha, max_rate, base_rate).unwrap();
    assert_eq!(
        ir,
        FixedI128::from_rational(53966913, 1_000_000_000).unwrap()
    );

    //utilization = 0.75, ir ~ 0.151126740, ir = 0.02/(1-0.75)^1.43 = 0,1452030649
    let total_debt = 75;
    let total_collateral: i128 = 100;
    let ir = calc_interest_rate(total_collateral, total_debt, alpha, max_rate, base_rate).unwrap();
    assert_eq!(
        ir,
        FixedI128::from_rational(151126740, 1_000_000_000).unwrap()
    );

    // utlization = 0.8, ir ~ 0.217230556, ir = 0.02/(1-0.8)^1.43 = 0,1997823429
    let total_debt: i128 = 80;
    let total_collateral: i128 = 100;
    let ir = calc_interest_rate(total_collateral, total_debt, alpha, max_rate, base_rate).unwrap();
    assert_eq!(
        ir,
        FixedI128::from_rational(217230556, 1_000_000_000).unwrap()
    );

    // utilization = 0.9, ir ~ 1,017163929, ir = 0.02/(1-0.9)^1.43 = 0,5383069608
    let total_debt: i128 = 90;
    let total_collateral: i128 = 100;
    let ir = calc_interest_rate(total_collateral, total_debt, alpha, max_rate, base_rate).unwrap();
    assert_eq!(
        ir,
        FixedI128::from_rational(1017163929, 1_000_000_000).unwrap()
    );

    //utilization = 0.95, ir - 5,00, ir = 0.02/(1-0.9)^1.43 = 1,117567356
    let total_debt: i128 = 95;
    let total_collateral: i128 = 100;
    let ir = calc_interest_rate(total_collateral, total_debt, alpha, max_rate, base_rate).unwrap();
    assert_eq!(
        ir,
        FixedI128::from_rational(5000000000u64, 1_000_000_000).unwrap()
    );
}