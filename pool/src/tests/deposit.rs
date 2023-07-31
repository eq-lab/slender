use crate::tests::sut::{fill_pool, init_pool, DAY};
use crate::*;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, Events, Ledger};
use soroban_sdk::{symbol_short, vec, IntoVal, Symbol};

extern crate std;

#[test]
fn should_require_authorized_caller() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::random(&env);
    let sut = init_pool(&env);
    let token_address = sut.token().address.clone();

    sut.token_admin().mint(&user, &1_000_000_000);
    sut.pool.deposit(&user, &token_address, &1_000_000_000);

    assert_eq!(
        env.auths().pop().map(|f| f.1.function).unwrap(),
        AuthorizedFunction::Contract((
            sut.pool.address.clone(),
            symbol_short!("deposit"),
            (user.clone(), token_address, 1_000_000_000i128).into_val(&env)
        )),
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn should_fail_when_pool_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::random(&env);
    let sut = init_pool(&env);
    let token_address = sut.token().address.clone();

    sut.pool.set_pause(&true);
    sut.pool.deposit(&user, &token_address, &1);

    // assert_eq!(
    //     sut.pool
    //         .try_deposit(&user, &token_address, &1)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::Paused
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_invalid_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::random(&env);
    let sut = init_pool(&env);
    let token_address = sut.token().address.clone();

    sut.pool.deposit(&user, &token_address, &-1);

    // assert_eq!(
    //     sut.pool
    //         .try_deposit(&user, &token_address, &-1)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::InvalidAmount
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_reserve_deactivated() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::random(&env);
    let sut = init_pool(&env);
    let token_address = sut.token().address.clone();

    sut.pool.set_reserve_status(&token_address, &false);
    sut.pool.deposit(&user, &token_address, &1);

    // assert_eq!(
    //     sut.pool
    //         .try_deposit(&user, &token_address, &1)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::NoActiveReserve
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_liq_cap_exceeded() {
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
fn should_change_user_config() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::random(&env);
    let sut = init_pool(&env);
    let token_address = sut.token().address.clone();

    sut.token_admin().mint(&user, &1_000_000_000);
    sut.pool.deposit(&user, &token_address, &1_000_000_000);

    let user_config = sut.pool.user_configuration(&user);
    let reserve = sut.pool.get_reserve(&token_address).unwrap();

    assert_eq!(
        user_config.is_using_as_collateral(&env, reserve.get_id()),
        true
    );
}

#[test]
fn should_change_balances() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::random(&env);
    let sut = init_pool(&env);
    let token_address = sut.token().address.clone();

    sut.token_admin().mint(&user, &10_000_000_000);
    sut.pool.deposit(&user, &token_address, &3_000_000_000);

    let stoken_underlying_balance = sut
        .pool
        .get_stoken_underlying_balance(&sut.s_token().address);
    let user_balance = sut.token().balance(&user);
    let user_stoken_balance = sut.s_token().balance(&user);

    assert_eq!(stoken_underlying_balance, 3_000_000_000);
    assert_eq!(user_balance, 7_000_000_000);
    assert_eq!(user_stoken_balance, 3_000_000_000);
}

#[test]
fn should_affect_coeffs() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (lender, _, debt_config) = fill_pool(&env, &sut);
    let debt_token = &debt_config.token.address;

    env.ledger().with_mut(|li| li.timestamp = DAY);

    let collat_coeff_prev = sut.pool.collat_coeff(&debt_token);
    let debt_coeff_prev = sut.pool.debt_coeff(&debt_token);

    sut.pool
        .deposit(&lender, &sut.reserves[1].token.address, &100_000_000);

    let collat_coeff = sut.pool.collat_coeff(&debt_token);
    let debt_coeff = sut.pool.debt_coeff(&debt_token);

    assert!(collat_coeff_prev < collat_coeff);
    assert!(debt_coeff_prev < debt_coeff);
}

#[test]
fn should_affect_account_data() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, _) = fill_pool(&env, &sut);

    let account_position_prev = sut.pool.account_position(&borrower);

    sut.pool
        .deposit(&borrower, &sut.reserves[0].token.address, &200_000_000);

    let account_position = sut.pool.account_position(&borrower);

    assert!(account_position_prev.discounted_collateral < account_position.discounted_collateral);
    assert!(account_position_prev.npv < account_position.npv);
}

#[test]
fn should_emit_events() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let user = Address::random(&env);
    let token_address = sut.token().address.clone();

    sut.token_admin().mint(&user, &10_000_000_000);
    assert_eq!(sut.token().balance(&user), 10_000_000_000);

    sut.pool.deposit(&user, &token_address, &5_000_000_000);

    let mut events = env.events().all();
    let event = events.pop_back_unchecked();

    assert_eq!(
        vec![&env, event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (
                    Symbol::new(&env, "reserve_used_as_coll_enabled"),
                    user.clone()
                )
                    .into_val(&env),
                (token_address.clone()).into_val(&env)
            ),
        ]
    );

    let event = events.pop_back_unchecked();

    assert_eq!(
        vec![&env, event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "deposit"), user.clone()).into_val(&env),
                (token_address, 5_000_000_000i128).into_val(&env)
            ),
        ]
    );
}
