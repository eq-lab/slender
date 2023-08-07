use common::FixedI128;
use pool_interface::{IRParams, InitReserveInput, ReserveData};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, Env};

use crate::rate::*;
use crate::tests::sut::{init_pool, DAY};

pub fn get_default_ir_params() -> IRParams {
    IRParams {
        alpha: 143,          //1.43
        initial_rate: 200,   //2%
        max_rate: 50000,     //500%
        scaling_coeff: 9000, //90%
    }
}

#[test]
fn should_return_initial_rate_when_utilization_is_zero() {
    let total_collateral = 1000;
    let total_debt = 0;
    let ir_params = get_default_ir_params();

    let ir = calc_interest_rate(total_collateral, total_debt, &ir_params);

    assert_eq!(ir, FixedI128::from_percentage(ir_params.initial_rate));
}

#[test]
fn should_return_max_rate_when_utilization_is_gte_one() {
    let total_collateral = 1;
    let total_debt = 1;
    let ir_params = get_default_ir_params();

    let ir = calc_interest_rate(total_collateral, total_debt, &ir_params);

    assert_eq!(ir, FixedI128::from_percentage(ir_params.max_rate));
}

#[test]
fn should_return_none_when_collateral_or_debt_is_negative() {
    let total_collateral = -1;
    let total_debt = 1;
    let ir_params = get_default_ir_params();

    let ir = calc_interest_rate(total_collateral, total_debt, &ir_params);

    assert!(ir.is_none());

    let total_collateral = 1;
    let total_debt = -1;
    let ir = calc_interest_rate(total_collateral, total_debt, &ir_params);

    assert!(ir.is_none());

    let total_collateral = -1;
    let total_debt = -1;
    let ir = calc_interest_rate(total_collateral, total_debt, &ir_params);

    assert!(ir.is_none());
}

#[test]
fn should_calc_interest_rate() {
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
fn should_calc_accrued_rate() {
    let prev_ar = FixedI128::ONE;
    let ir = FixedI128::from_percentage(2000).unwrap(); // 20%

    //ar = 1 * (1 + 20/100 * 24 * 60 * 60/31_557_600) = 1,0005475702
    assert_eq!(
        calc_next_accrued_rate(prev_ar, ir, DAY),
        Some(FixedI128::from_inner(1000547570))
    );
}

#[test]
fn should_calc_borrower_and_lender_rates() {
    let env = &Env::default();
    let total_collateral = 100;
    let total_debt = 20;

    let input = InitReserveInput {
        s_token_address: Address::random(env),
        debt_token_address: Address::random(env),
    };
    let reserve_data = ReserveData::new(env, input);
    let ir_params = get_default_ir_params();

    let accrued_rates =
        calc_accrued_rates(total_collateral, total_debt, DAY, ir_params, &reserve_data).unwrap();

    //debt_ir = 0,027517810
    assert_eq!(accrued_rates.borrower_ir.into_inner(), 27517810);
    // collat_ar = 1*(1 + 0,0275176482 * 24*60*60/31_557_600) = 1,0000753392
    assert_eq!(accrued_rates.borrower_ar.into_inner(), 1000075339);

    //lender_ir = 0,024766029
    assert_eq!(accrued_rates.lender_ir.into_inner(), 24766029);
    //collat_ar = 1*(1 + 0.9*0,0275176482 * 24*60*60/31_557_600) = 1,0000678053
    assert_eq!(accrued_rates.lender_ar.into_inner(), 1000067805);
}

#[test]
fn should_fail_when_collateral_is_zero() {
    let env = &Env::default();
    let total_collateral = 0;
    let total_debt = 100;

    let input = InitReserveInput {
        s_token_address: Address::random(env),
        debt_token_address: Address::random(env),
    };
    let reserve_data = ReserveData::new(env, input);
    let ir_params = get_default_ir_params();

    let mb_accrued_rates =
        calc_accrued_rates(total_collateral, total_debt, DAY, ir_params, &reserve_data);
    assert!(mb_accrued_rates.is_none());
}

#[test]
fn should_update_rates_over_time() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let debt_asset_1 = sut.reserves[1].token.address.clone();

    let lender = Address::random(&env);
    let borrower = Address::random(&env);

    for r in sut.reserves.iter() {
        r.token_admin.mint(&lender, &1_000_000_000);
        r.token_admin.mint(&borrower, &1_000_000_000);
    }

    for r in sut.reserves.iter() {
        sut.pool.deposit(&lender, &r.token.address, &100_000_000);
    }

    sut.pool
        .deposit(&borrower, &sut.reserves[0].token.address, &100_000_000);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

    assert_eq!(s_token_underlying_supply, 100_000_000);

    // ensure that zero elapsed time doesn't change AR coefficients
    {
        let reserve_before = sut.pool.get_reserve(&debt_asset_1).unwrap();
        sut.pool.borrow(&borrower, &debt_asset_1, &40_000_000);

        let updated_reserve = sut.pool.get_reserve(&debt_asset_1).unwrap();
        assert_eq!(updated_reserve.lender_ar, reserve_before.lender_ar);
        assert_eq!(updated_reserve.borrower_ar, reserve_before.borrower_ar);
        assert_eq!(
            reserve_before.last_update_timestamp,
            updated_reserve.last_update_timestamp
        );
        assert_eq!(s_token_underlying_supply, 60_000_000);
    }

    // shift time to
    env.ledger().with_mut(|li| li.timestamp = DAY);

    //second deposit by lender of debt asset
    sut.pool.deposit(&lender, &debt_asset_1, &100_000_000);

    let updated = sut.pool.get_reserve(&debt_asset_1).unwrap();
    let ir_params = sut.pool.ir_params().unwrap();
    let debt_ir = calc_interest_rate(100_000_000, 40_000_000, &ir_params).unwrap();
    let lender_ir = debt_ir
        .checked_mul(FixedI128::from_percentage(ir_params.scaling_coeff).unwrap())
        .unwrap();

    let elapsed_time = env.ledger().timestamp();

    let coll_ar = calc_next_accrued_rate(FixedI128::ONE, lender_ir, elapsed_time)
        .unwrap()
        .into_inner();
    let debt_ar = calc_next_accrued_rate(FixedI128::ONE, debt_ir, elapsed_time)
        .unwrap()
        .into_inner();

    let _collat_coeff = sut.pool.collat_coeff(&debt_asset_1);
    let _debt_coeff = sut.pool.debt_coeff(&debt_asset_1);

    assert_eq!(updated.lender_ar, coll_ar);
    assert_eq!(updated.borrower_ar, debt_ar);
    assert_eq!(updated.lender_ir, lender_ir.into_inner());
    assert_eq!(updated.borrower_ir, debt_ir.into_inner());
}
