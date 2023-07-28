use crate::tests::sut::{fill_pool, init_pool};
use crate::*;
use common::FixedI128;
use soroban_sdk::testutils::{Address as _, Events, Ledger};
use soroban_sdk::{vec, IntoVal, Symbol};

extern crate std;

// todo: check events /Artur
// todo: check all errors /Artur
// todo: check user_config /Artur
// todo: 1 test per execution branch /Artur
// todo: repay /Artur
// todo: separate test to validate budgets /Artur

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn deposit_pool_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::random(&env);
    let sut = init_pool(&env);
    let token_address = sut.token().address.clone();

    sut.pool.set_pause(&true);

    sut.pool.deposit(&user, &token_address, &1);
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn deposit_invalid_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::random(&env);
    let sut = init_pool(&env);
    let token_address = sut.token().address.clone();

    sut.pool.deposit(&user, &token_address, &-1);
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn deposit_reserve_deactivated() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::random(&env);
    let sut = init_pool(&env);
    let token_address = sut.token().address.clone();

    sut.pool.deposit(&user, &token_address, &-1);
}

#[test]
fn deposit() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let token = &sut.reserves[0].token;
    let token_admin = &sut.reserves[0].token_admin;
    let s_token = &sut.reserves[0].s_token;

    for i in 0..10 {
        let user = Address::random(&env);
        let initial_balance = 1_000_000_000;
        token_admin.mint(&user, &1_000_000_000);
        assert_eq!(token.balance(&user), initial_balance);

        let deposit_amount = 10_000;
        let lender_accrued_rate = Some(FixedI128::ONE.into_inner() + i * 100_000_000);

        assert_eq!(
            sut.pool
                .set_accrued_rates(&token.address, &lender_accrued_rate, &None),
            ()
        );
        let collat_coeff = sut.pool.collat_coeff(&token.address);
        sut.pool.deposit(&user, &token.address, &deposit_amount);

        assert_eq!(
            s_token.balance(&user),
            deposit_amount * FixedI128::ONE.into_inner() / collat_coeff
        );
        assert_eq!(token.balance(&user), initial_balance - deposit_amount);

        let last = env.events().all().pop_back_unchecked();
        assert_eq!(
            vec![&env, last],
            vec![
                &env,
                (
                    sut.pool.address.clone(),
                    (Symbol::new(&env, "reserve_used_as_coll_enabled"), user).into_val(&env),
                    (token.address.clone()).into_val(&env)
                ),
            ]
        );
    }
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn deposit_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let user1 = Address::random(&env);

    //TODO: check error after soroban fix
    let deposit_amount = 0;
    sut.pool
        .deposit(&user1, &sut.reserves[0].token.address, &deposit_amount);

    // assert_eq!(
    //     sut.pool
    //         .try_deposit(&user1, &sut.reserves[0].token.address, &deposit_amount,)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::InvalidAmount
    // )
}

#[test]
fn deposit_non_active_reserve() {
    //TODO: implement when possible
}

#[test]
fn deposit_frozen_() {
    //TODO: implement when possible
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn deposit_should_fail_when_exceeded_liq_cap() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let token = &sut.reserves[0].token;
    let token_admin = &sut.reserves[0].token_admin;
    let s_token = &sut.reserves[0].s_token;
    let decimals = s_token.decimals();

    let user = Address::random(&env);
    let initial_balance = 1_000_000_000 * 10i128.pow(decimals);

    token_admin.mint(&user, &initial_balance);
    assert_eq!(token.balance(&user), initial_balance);

    let deposit_amount = initial_balance;
    sut.pool.deposit(&user, &token.address, &deposit_amount);

    // assert_eq!(
    //     sut.pool
    //         .try_deposit(&user, &token.address, &deposit_amount)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::LiqCapExceeded
    // )
}

#[test]
fn deposit_should_mint_s_token() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let (lender, _borrower, debt_config) = fill_pool(&env, &sut);
    let debt_token = &debt_config.token.address;
    // shift time to one day
    env.ledger().with_mut(|li| {
        li.timestamp = 24 * 60 * 60 // one day
    });

    let stoken_supply = debt_config.s_token.total_supply();
    let lender_stoken_balance_before = debt_config.s_token.balance(&lender);
    let deposit_amount = 10_000;
    sut.pool
        .deposit(&lender, &sut.reserves[1].token.address, &deposit_amount);

    let _reserve = sut.pool.get_reserve(&debt_token).unwrap();
    let collat_coeff = sut.pool.collat_coeff(&debt_token);
    let _debt_coeff = sut.pool.debt_coeff(&debt_token);

    let expected_stoken_amount = FixedI128::from_inner(collat_coeff)
        .recip_mul_int(deposit_amount)
        .unwrap();

    assert_eq!(
        debt_config.s_token.balance(&lender),
        lender_stoken_balance_before + expected_stoken_amount
    );
    assert_eq!(
        debt_config.s_token.total_supply(),
        stoken_supply + expected_stoken_amount
    );
    let collat_coeff_prev = sut.pool.collat_coeff(&debt_token);
    let debt_coeff_prev = sut.pool.debt_coeff(&debt_token);
    // shift time to one day
    env.ledger().with_mut(|li| {
        li.timestamp = 2 * 24 * 60 * 60 // one day
    });

    let collat_coeff = sut.pool.collat_coeff(&debt_token);
    let debt_coeff = sut.pool.debt_coeff(&debt_token);

    assert!(collat_coeff_prev < collat_coeff);
    assert!(debt_coeff_prev < debt_coeff);
}
