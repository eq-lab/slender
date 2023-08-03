use crate::rate::{calc_interest_rate, calc_next_accrued_rate};
use crate::tests::sut::{
    create_pool_contract, create_price_feed_contract, create_s_token_contract,
    create_token_contract, init_pool, DAY,
};
use crate::*;
use common::FixedI128;
use price_feed_interface::PriceFeedClient;
use soroban_sdk::testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke};
use soroban_sdk::{vec, IntoVal};

use super::sut::fill_pool;

extern crate std;

#[test]
fn init_reserve() {
    let env = Env::default();

    let admin = Address::random(&env);
    let token_admin = Address::random(&env);

    let (underlying_token, _) = create_token_contract(&env, &token_admin);
    let (debt_token, _) = create_token_contract(&env, &token_admin);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin);
    let s_token = create_s_token_contract(&env, &pool.address, &underlying_token.address);
    assert!(pool.get_reserve(&underlying_token.address).is_none());

    let init_reserve_input = InitReserveInput {
        s_token_address: s_token.address.clone(),
        debt_token_address: debt_token.address.clone(),
    };

    assert_eq!(
        pool.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &pool.address,
                fn_name: "init_reserve",
                args: (&underlying_token.address, init_reserve_input.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .init_reserve(&underlying_token.address, &init_reserve_input),
        ()
    );

    let reserve = pool.get_reserve(&underlying_token.address).unwrap();

    assert!(pool.get_reserve(&underlying_token.address).is_some());
    assert_eq!(init_reserve_input.s_token_address, reserve.s_token_address);
    assert_eq!(
        init_reserve_input.debt_token_address,
        reserve.debt_token_address
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn init_reserve_second_time() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let init_reserve_input = InitReserveInput {
        s_token_address: sut.s_token().address.clone(),
        debt_token_address: sut.debt_token().address.clone(),
    };

    //TODO: check error after soroban fix
    sut.pool
        .init_reserve(&sut.token().address, &init_reserve_input);

    // assert_eq!(
    //     sut.pool
    //         .try_init_reserve(&sut.token().address, &init_reserve_input)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::ReserveAlreadyInitialized
    // )
}

#[test]
fn init_reserve_when_pool_not_initialized() {
    let env = Env::default();

    let admin = Address::random(&env);
    let token_admin = Address::random(&env);

    let (underlying_token, _) = create_token_contract(&env, &token_admin);
    let (debt_token, _) = create_token_contract(&env, &token_admin);

    let pool: LendingPoolClient<'_> =
        LendingPoolClient::new(&env, &env.register_contract(None, LendingPool));
    let s_token = create_s_token_contract(&env, &pool.address, &underlying_token.address);
    assert!(pool.get_reserve(&underlying_token.address).is_none());

    let init_reserve_input = InitReserveInput {
        s_token_address: s_token.address.clone(),
        debt_token_address: debt_token.address.clone(),
    };

    assert_eq!(
        pool.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &pool.address,
                fn_name: "init_reserve",
                args: (&underlying_token.address, init_reserve_input.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .try_init_reserve(&underlying_token.address, &init_reserve_input)
        .unwrap_err()
        .unwrap(),
        Error::Uninitialized
    );
}

#[test]
fn set_ir_params() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let ir_params_input = IRParams {
        alpha: 144,
        initial_rate: 201,
        max_rate: 50_001,
        scaling_coeff: 9_001,
    };

    sut.pool.set_ir_params(&ir_params_input);

    let ir_params = sut.pool.ir_params().unwrap();

    assert_eq!(ir_params_input.alpha, ir_params.alpha);
    assert_eq!(ir_params_input.initial_rate, ir_params.initial_rate);
    assert_eq!(ir_params_input.max_rate, ir_params.max_rate);
    assert_eq!(ir_params_input.scaling_coeff, ir_params.scaling_coeff);
}

#[test]
fn set_price_feed() {
    let env = Env::default();

    let admin = Address::random(&env);
    let asset_1 = Address::random(&env);
    let asset_2 = Address::random(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin);
    let price_feed: PriceFeedClient<'_> = create_price_feed_contract(&env);
    let assets = vec![&env, asset_1.clone(), asset_2.clone()];

    assert!(pool.price_feed(&asset_1.clone()).is_none());
    assert!(pool.price_feed(&asset_2.clone()).is_none());

    assert_eq!(
        pool.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &pool.address,
                fn_name: "set_price_feed",
                args: (&price_feed.address, assets.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_price_feed(&price_feed.address, &assets.clone()),
        ()
    );

    assert_eq!(pool.price_feed(&asset_1).unwrap(), price_feed.address);
    assert_eq!(pool.price_feed(&asset_2).unwrap(), price_feed.address);
}

#[test]
fn user_operation_should_update_ar_coeffs() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let debt_asset_1 = sut.reserves[1].token.address.clone();

    let lender = Address::random(&env);
    let borrower_1 = Address::random(&env);
    let borrow_amount = 40_000_000;

    //init pool with one borrower and one lender
    let initial_amount: i128 = 1_000_000_000;
    for r in sut.reserves.iter() {
        r.token_admin.mint(&lender, &initial_amount);
        r.token_admin.mint(&borrower_1, &initial_amount);
    }

    //lender deposit all tokens
    let deposit_amount = 100_000_000;
    for r in sut.reserves.iter() {
        sut.pool.deposit(&lender, &r.token.address, &deposit_amount);
    }

    sut.pool
        .deposit(&borrower_1, &sut.reserves[0].token.address, &deposit_amount);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

    assert_eq!(s_token_underlying_supply, 100_000_000);

    // ensure that zero elapsed time doesn't change AR coefficients
    {
        let reserve_before = sut.pool.get_reserve(&debt_asset_1).unwrap();
        sut.pool.borrow(&borrower_1, &debt_asset_1, &borrow_amount);

        let s_token_underlying_supply = sut
            .pool
            .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

        let updated_reserve = sut.pool.get_reserve(&debt_asset_1).unwrap();
        assert_eq!(
            updated_reserve.lender_accrued_rate,
            reserve_before.lender_accrued_rate
        );
        assert_eq!(
            updated_reserve.borrower_accrued_rate,
            reserve_before.borrower_accrued_rate
        );
        assert_eq!(
            reserve_before.last_update_timestamp,
            updated_reserve.last_update_timestamp
        );
        assert_eq!(s_token_underlying_supply, 60_000_000);
    }

    // shift time to
    env.ledger().with_mut(|li| {
        li.timestamp = 24 * 60 * 60 // one day
    });

    //second deposit by lender of debt asset
    sut.pool.deposit(&lender, &debt_asset_1, &deposit_amount);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

    let updated = sut.pool.get_reserve(&debt_asset_1).unwrap();
    let ir_params = sut.pool.ir_params().unwrap();
    let debt_ir = calc_interest_rate(deposit_amount, borrow_amount, &ir_params).unwrap();
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

    assert_eq!(updated.lender_accrued_rate, coll_ar);
    assert_eq!(updated.borrower_accrued_rate, debt_ar);
    assert_eq!(updated.lender_ir, lender_ir.into_inner());
    assert_eq!(updated.borrower_ir, debt_ir.into_inner());
    assert_eq!(s_token_underlying_supply, 160_000_000);
}

#[test]
fn collateral_coeff_test() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    env.budget().reset_unlimited();

    let (_lender, borrower, debt_config) = fill_pool(&env, &sut, true);
    let initial_collat_coeff = sut.pool.collat_coeff(&debt_config.token.address);
    std::println!("initial_collat_coeff={}", initial_collat_coeff);

    env.ledger().with_mut(|l| {
        l.timestamp = 2 * DAY;
    });

    let borrow_amount = 50_000;
    sut.pool
        .borrow(&borrower, &debt_config.token.address, &borrow_amount);
    let reserve = sut.pool.get_reserve(&debt_config.token.address).unwrap();

    let collat_ar = FixedI128::from_inner(reserve.lender_accrued_rate);
    let s_token_supply = debt_config.s_token.total_supply();
    let balance = debt_config.token.balance(&debt_config.s_token.address);
    let debt_token_suply = debt_config.debt_token.total_supply();

    let expected_collat_coeff = FixedI128::from_rational(
        balance + collat_ar.mul_int(debt_token_suply).unwrap(),
        s_token_supply,
    )
    .unwrap()
    .into_inner();

    let collat_coeff = sut.pool.collat_coeff(&debt_config.token.address);
    assert_eq!(collat_coeff, expected_collat_coeff);

    // shift time to 8 days
    env.ledger().with_mut(|l| {
        l.timestamp = 10 * DAY;
    });

    let elapsed_time = 8 * DAY;
    let collat_ar = calc_next_accrued_rate(
        collat_ar,
        FixedI128::from_inner(reserve.lender_ir),
        elapsed_time,
    )
    .unwrap();
    let expected_collat_coeff = FixedI128::from_rational(
        balance + collat_ar.mul_int(debt_token_suply).unwrap(),
        s_token_supply,
    )
    .unwrap()
    .into_inner();

    let collat_coeff = sut.pool.collat_coeff(&debt_config.token.address);
    assert_eq!(collat_coeff, expected_collat_coeff);
    std::println!("collat_coeff={}", collat_coeff);
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn liquidity_cap_test() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    env.budget().reset_unlimited();

    let (lender, _borrower, debt_config) = fill_pool(&env, &sut, true);

    let token_one = 10_i128.pow(debt_config.token.decimals());
    let liq_bonus = 11000; //110%
    let liq_cap = 1_000_000 * 10_i128.pow(debt_config.token.decimals()); // 1M
    let discount = 6000; //60%
    let util_cap = 9000; //90%

    sut.pool.configure_as_collateral(
        &debt_config.token.address,
        &CollateralParamsInput {
            liq_bonus,
            liq_cap,
            discount,
            util_cap,
        },
    );

    //TODO: check error after soroban fix
    let deposit_amount = 1_000_000 * token_one;
    sut.pool
        .deposit(&lender, &debt_config.token.address, &deposit_amount);

    // assert_eq!(
    //     sut.pool
    //         .try_deposit(&lender, &debt_config.token.address, &deposit_amount)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::LiqCapExceeded
    // );
}

#[test]
fn stoken_balance_not_changed_when_direct_transfer_to_underlying_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let lender = Address::random(&env);

    env.budget().reset_unlimited();

    sut.reserves[0].token_admin.mint(&lender, &2_000_000_000);
    sut.pool
        .deposit(&lender, &sut.reserves[0].token.address, &1_000_000_000);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[0].s_token.address);

    assert_eq!(s_token_underlying_supply, 1_000_000_000);

    sut.reserves[0]
        .token
        .transfer(&lender, &sut.reserves[0].s_token.address, &1_000_000_000);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[0].s_token.address);

    assert_eq!(s_token_underlying_supply, 1_000_000_000);
}
