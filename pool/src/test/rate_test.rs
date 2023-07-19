use common::FixedI128;
use pool_interface::{IRParams, InitReserveInput, ReserveData};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

use crate::rate::*;

pub fn get_default_ir_params() -> IRParams {
    IRParams {
        alpha: 143,          //1.43
        initial_rate: 200,   //2%
        max_rate: 50000,     //500%
        scaling_coeff: 9000, //90%
    }
}

#[test]
fn calc_ir_utilization_is_zero() {
    let total_collateral = 1000;
    let total_debt = 0;
    let ir_params = get_default_ir_params();

    let ir = calc_interest_rate(total_collateral, total_debt, &ir_params);

    assert_eq!(ir, FixedI128::from_percentage(ir_params.initial_rate));
}

#[test]
fn calc_ir_utilization_is_one() {
    let total_collateral = 1;
    let total_debt = 1;
    let ir_params = get_default_ir_params();

    let ir = calc_interest_rate(total_collateral, total_debt, &ir_params);

    assert_eq!(ir, FixedI128::from_percentage(ir_params.max_rate));
}

#[test]
fn calc_ir() {
    let ir_params = get_default_ir_params();

    //utilization = 0.2, ir ~ 0.027517810, ir = 0.02/(1-0.2)^1.43 = 0,0275176482
    let total_debt = 20;
    let total_collateral: i128 = 100;
    let ir = calc_interest_rate(total_collateral, total_debt, &ir_params).unwrap();
    assert_eq!(
        ir,
        FixedI128::from_rational(27517810, 1_000_000_000).unwrap()
    );

    //utilization = 0.5, ir ~ 0.053966913, ir = 0.02/(1 - 0.5)^1.43 = 0,0538893431
    let total_debt = 50;
    let total_collateral: i128 = 100;
    let ir = calc_interest_rate(total_collateral, total_debt, &ir_params).unwrap();
    assert_eq!(
        ir,
        FixedI128::from_rational(53966913, 1_000_000_000).unwrap()
    );

    //utilization = 0.75, ir ~ 0.151126740, ir = 0.02/(1-0.75)^1.43 = 0,1452030649
    let total_debt = 75;
    let total_collateral: i128 = 100;
    let ir = calc_interest_rate(total_collateral, total_debt, &ir_params).unwrap();
    assert_eq!(
        ir,
        FixedI128::from_rational(151126740, 1_000_000_000).unwrap()
    );

    // utlization = 0.8, ir ~ 0.217230556, ir = 0.02/(1-0.8)^1.43 = 0,1997823429
    let total_debt: i128 = 80;
    let total_collateral: i128 = 100;
    let ir = calc_interest_rate(total_collateral, total_debt, &ir_params).unwrap();
    assert_eq!(
        ir,
        FixedI128::from_rational(217230556, 1_000_000_000).unwrap()
    );

    // utilization = 0.9, ir ~ 1,017163929, ir = 0.02/(1-0.9)^1.43 = 0,5383069608
    let total_debt: i128 = 90;
    let total_collateral: i128 = 100;
    let ir = calc_interest_rate(total_collateral, total_debt, &ir_params).unwrap();
    assert_eq!(
        ir,
        FixedI128::from_rational(1017163929, 1_000_000_000).unwrap()
    );

    //utilization = 0.95, ir - 5,00, ir = 0.02/(1-0.9)^1.43 = 1,117567356
    let total_debt: i128 = 95;
    let total_collateral: i128 = 100;
    let ir = calc_interest_rate(total_collateral, total_debt, &ir_params).unwrap();
    assert_eq!(
        ir,
        FixedI128::from_rational(5000000000u64, 1_000_000_000).unwrap()
    );
}

#[test]
fn test_calc_accrued_rate() {
    let prev_ar = FixedI128::ONE;
    let ir = FixedI128::from_percentage(2000).unwrap(); // 20%
    let one_day: u64 = 24 * 60 * 60;

    //ar = 1 * (1 + 20/100 * 24 * 60 * 60/31_557_600) = 1,0005475702
    assert_eq!(
        calc_accrued_rate_coeff(prev_ar, ir, one_day),
        Some(FixedI128::from_inner(1000547570))
    );
}

#[test]
fn test_update_accrued_rates() {
    let env = &Env::default();
    let total_collateral = 100;
    let total_debt = 20;
    let one_day = 24 * 60 * 60;

    let input = InitReserveInput {
        s_token_address: Address::random(env),
        debt_token_address: Address::random(env),
    };
    let reserve_data = ReserveData::new(env, input);
    let ir_params = get_default_ir_params();

    let accrued_rates = calc_accrued_rates(
        total_collateral,
        total_debt,
        one_day,
        ir_params,
        &reserve_data,
    )
    .unwrap();

    //debt_ir = 0,027517810
    assert_eq!(accrued_rates.debt_ir.into_inner(), 27517810);
    // collat_ar = 1*(1 + 0,0275176482 * 24*60*60/31_557_600) = 1,0000753392
    assert_eq!(accrued_rates.debt_accrued_rate.into_inner(), 1000075339);

    //lend_ir = 0,024766029
    assert_eq!(accrued_rates.lend_ir.into_inner(), 24766029);
    //collat_ar = 1*(1 + 0.9*0,0275176482 * 24*60*60/31_557_600) = 1,0000678053
    assert_eq!(accrued_rates.collat_accrued_rate.into_inner(), 1000067805);
}

#[test]
fn update_accrued_rates_should_fail() {
    let env = &Env::default();
    let total_collateral = 0;
    let total_debt = 0;
    let one_day = 24 * 60 * 60;

    let input = InitReserveInput {
        s_token_address: Address::random(env),
        debt_token_address: Address::random(env),
    };
    let reserve_data = ReserveData::new(env, input);
    let ir_params = get_default_ir_params();

    let mb_accrued_rates = calc_accrued_rates(
        total_collateral,
        total_debt,
        one_day,
        ir_params,
        &reserve_data,
    );
    assert!(mb_accrued_rates.is_none());
}
