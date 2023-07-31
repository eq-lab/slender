use crate::rate::{calc_interest_rate, calc_next_accrued_rate};
use crate::tests::sut::{
    create_pool_contract, create_price_feed_contract, create_s_token_contract,
    create_token_contract, fill_pool_two, init_pool, DAY,
};
use crate::*;
use common::FixedI128;
use debt_token_interface::DebtTokenClient;
use price_feed_interface::PriceFeedClient;
use s_token_interface::STokenClient;
use soroban_sdk::symbol_short;
use soroban_sdk::testutils::{Address as _, Events, Ledger, MockAuth, MockAuthInvoke};
use soroban_sdk::{vec, IntoVal, Symbol};

use super::sut::fill_pool;

extern crate std;

#[test]
fn withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let (lender, _borrower, debt_config) = fill_pool(&env, &sut);
    let debt_token = &debt_config.token.address;

    env.ledger().with_mut(|li| {
        li.timestamp = 60 * DAY;
    });

    let lender_s_token_balance = debt_config.s_token.balance(&lender);
    let s_token_supply = debt_config.s_token.total_supply();
    assert_eq!(s_token_supply, 100000000);
    assert_eq!(lender_s_token_balance, 100000000);

    let withdraw_amount = 1_000_000;
    sut.pool
        .withdraw(&lender, debt_token, &withdraw_amount, &lender);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&debt_config.s_token.address);

    let lender_underlying_balance = debt_config.token.balance(&lender);
    let lender_s_token_balance = debt_config.s_token.balance(&lender);
    let s_token_supply = debt_config.s_token.total_supply();

    assert_eq!(lender_underlying_balance, 901000000);
    assert_eq!(lender_s_token_balance, 99002451);
    assert_eq!(s_token_supply, 99002451);
    assert_eq!(s_token_underlying_supply, 59_000_000);
}

#[test]
fn withdraw_full() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let (lender, _lender, _borrower, debt_config) = fill_pool_two(&env, &sut);
    let debt_token = &debt_config.token.address;

    env.ledger().with_mut(|li| {
        li.timestamp = 60 * DAY;
    });

    let lender_s_token_balance = debt_config.s_token.balance(&lender);
    let s_token_supply = debt_config.s_token.total_supply();
    assert_eq!(s_token_supply, 200000000);
    assert_eq!(lender_s_token_balance, 100000000);

    let withdraw_amount = i128::MAX;
    sut.pool
        .withdraw(&lender, debt_token, &withdraw_amount, &lender);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&debt_config.s_token.address);

    let lender_underlying_balance = debt_config.token.balance(&lender);
    let lender_s_token_balance = debt_config.s_token.balance(&lender);
    let s_token_supply = debt_config.s_token.total_supply();

    assert_eq!(lender_underlying_balance, 1000081366);
    assert_eq!(lender_s_token_balance, 0);
    assert_eq!(s_token_supply, 100000000);
    assert_eq!(s_token_underlying_supply, 59_918_634);
}

#[test]
fn withdraw_base() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let user1 = Address::random(&env);
    let user2 = Address::random(&env);

    let initial_balance = 1_000_000_000;
    sut.token_admin().mint(&user1, &1_000_000_000);
    assert_eq!(sut.token().balance(&user1), initial_balance);

    let deposit_amount = 10000;
    sut.pool
        .deposit(&user1, &sut.token().address, &deposit_amount);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.s_token().address);

    assert_eq!(sut.s_token().balance(&user1), deposit_amount);
    assert_eq!(
        sut.token().balance(&user1),
        initial_balance - deposit_amount
    );
    assert_eq!(sut.token().balance(&sut.s_token().address), deposit_amount);
    assert_eq!(s_token_underlying_supply, 10_000);

    let amount_to_withdraw = 3500;
    sut.pool
        .withdraw(&user1, &sut.token().address, &amount_to_withdraw, &user2);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.s_token().address);

    assert_eq!(sut.token().balance(&user2), amount_to_withdraw);
    assert_eq!(
        sut.s_token().balance(&user1),
        deposit_amount - amount_to_withdraw
    );
    assert_eq!(
        sut.token().balance(&sut.s_token().address),
        deposit_amount - amount_to_withdraw
    );
    assert_eq!(s_token_underlying_supply, 6_500);

    let withdraw_event = env.events().all().pop_back_unchecked();
    assert_eq!(
        vec![&env, withdraw_event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (symbol_short!("withdraw"), &user1).into_val(&env),
                (&user2, &sut.token().address, amount_to_withdraw).into_val(&env)
            ),
        ]
    );

    sut.pool
        .withdraw(&user1, &sut.token().address, &i128::MAX, &user2);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.s_token().address);

    assert_eq!(sut.token().balance(&user2), deposit_amount);
    assert_eq!(sut.s_token().balance(&user1), 0);
    assert_eq!(sut.token().balance(&sut.s_token().address), 0);
    assert_eq!(s_token_underlying_supply, 0);

    let withdraw_event = env.events().all().pop_back_unchecked();
    assert_eq!(
        vec![&env, withdraw_event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (symbol_short!("withdraw"), &user1).into_val(&env),
                (
                    &user2,
                    sut.token().address.clone(),
                    deposit_amount - amount_to_withdraw
                )
                    .into_val(&env)
            ),
        ]
    );

    let coll_disabled_event = env
        .events()
        .all()
        .get(env.events().all().len() - 4)
        .unwrap();
    assert_eq!(
        vec![&env, coll_disabled_event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "reserve_used_as_coll_disabled"), &user1).into_val(&env),
                (sut.token().address.clone()).into_val(&env)
            ),
        ]
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn withdraw_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let _token1 = &sut.reserves[0].token;
    let token2 = &sut.reserves[1].token;
    let token2_admin = &sut.reserves[1].token_admin;

    let user1 = Address::random(&env);
    token2_admin.mint(&user1, &1000);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    sut.pool.deposit(&user1, &token2.address, &1000);

    let withdraw_amount = 0;

    sut.pool
        .withdraw(&user1, &token2.address, &withdraw_amount, &user1);
    //TODO: check error after soroban fix
    // assert_eq!(
    //     sut.pool
    //         .try_withdraw(&user1, &token1.address, &withdraw_amount, &user1),
    //     Err(Ok(Error::InvalidAmount))
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn withdraw_more_than_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let token = &sut.reserves[0].token;
    let token_admin = &sut.reserves[0].token_admin;

    let user1 = Address::random(&env);

    let initial_balance = 1_000_000_000;
    token_admin.mint(&user1, &1_000_000_000);
    assert_eq!(token.balance(&user1), initial_balance);

    env.budget().reset_unlimited();

    let deposit_amount = 1000;
    sut.pool.deposit(&user1, &token.address, &deposit_amount);

    let withdraw_amount = 2000;

    //TODO: check error after soroban fix
    sut.pool
        .withdraw(&user1, &token.address, &withdraw_amount, &user1);

    // assert_eq!(
    //     sut.pool
    //         .try_withdraw(&user1, &token.address, &withdraw_amount, &user1)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::NotEnoughAvailableUserBalance
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn withdraw_unknown_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let user1 = Address::random(&env);
    let unknown_asset = &sut.reserves[0].debt_token.address;

    //TODO: check error after soroban fix
    let withdraw_amount = 1000;
    sut.pool
        .withdraw(&user1, unknown_asset, &withdraw_amount, &user1);

    // assert_eq!(
    //     sut.pool
    //         .try_withdraw(&user1, unknown_asset, &withdraw_amount, &user1)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::NoReserveExistForAsset
    // )
}

#[test]
fn withdraw_non_active_reserve() {
    //TODO: implement when it possible
}

#[test]
fn withdraw_should_burn_s_token() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let (lender, _borrower, debt_config) = fill_pool(&env, &sut);

    // shift time to one month
    env.ledger().with_mut(|li| {
        li.timestamp = 30 * 24 * 60 * 60 // one month
    });

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&debt_config.s_token.address);

    assert_eq!(s_token_underlying_supply, 60_000_000);

    let stoken_supply = debt_config.s_token.total_supply();
    let lender_stoken_balance_before = debt_config.s_token.balance(&lender);
    let withdraw_amount = 553_000;
    sut.pool.withdraw(
        &lender,
        &debt_config.token.address,
        &withdraw_amount,
        &lender,
    );

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&debt_config.s_token.address);

    let collat_coeff = FixedI128::from_inner(sut.pool.collat_coeff(&debt_config.token.address));
    let expected_burned_stoken = collat_coeff.recip_mul_int(withdraw_amount).unwrap();

    assert_eq!(
        debt_config.s_token.balance(&lender),
        lender_stoken_balance_before - expected_burned_stoken
    );
    assert_eq!(
        debt_config.s_token.total_supply(),
        stoken_supply - expected_burned_stoken
    );
    assert_eq!(s_token_underlying_supply, 59_447_000);
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn test_withdraw_bad_position() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    env.budget().reset_unlimited();

    let collateral = &sut.reserves[0].token;
    let collateral_admin = &sut.reserves[0].token_admin;

    let debt = &sut.reserves[1].token;
    let debt_admin = &sut.reserves[1].token_admin;

    let user = Address::random(&env);
    let lender = Address::random(&env);
    let deposit = 1_000_000_000;
    collateral_admin.mint(&user, &1_000_000_000);
    sut.pool.deposit(&user, &collateral.address, &deposit);
    let discount = sut
        .pool
        .get_reserve(&collateral.address)
        .expect("Reserve")
        .configuration
        .discount;
    let debt_amount = FixedI128::from_percentage(discount)
        .unwrap()
        .mul_int(deposit)
        .unwrap();
    debt_admin.mint(&lender, &deposit);
    sut.pool.deposit(&lender, &debt.address, &deposit);

    sut.pool.borrow(&user, &debt.address, &(debt_amount - 1));

    sut.pool
        .withdraw(&user, &collateral.address, &(deposit / 2), &user);

    //TODO: check error after soroban fix
    // assert_eq!(
    //     sut.pool
    //         .try_withdraw(&user, &collateral.address, &(deposit / 2), &user)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::BadPosition
    // );
}
