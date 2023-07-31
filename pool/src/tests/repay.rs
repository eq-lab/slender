use crate::tests::sut::{fill_pool, init_pool, DAY};
use crate::*;
use soroban_sdk::testutils::{Events, Ledger};
use soroban_sdk::{vec, IntoVal, Symbol};

extern crate std;

#[test]
fn should_partially_repay() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, debt_config) = fill_pool(&env, &sut);
    let debt_token = &debt_config.token.address;
    let stoken_token = &debt_config.s_token.address;

    env.ledger().with_mut(|li| li.timestamp = DAY);
    let treasury_address = sut.pool.treasury().clone();

    let stoken_underlying_balance = sut.pool.get_stoken_underlying_balance(&stoken_token);
    let user_balance = debt_config.token.balance(&borrower);
    let treasury_balance = debt_config.token.balance(&treasury_address);
    let user_debt_balance = debt_config.debt_token.balance(&borrower);

    assert_eq!(stoken_underlying_balance, 60_000_000);
    assert_eq!(user_balance, 1_040_000_000);
    assert_eq!(treasury_balance, 0);
    assert_eq!(user_debt_balance, 40_000_000);

    sut.pool.deposit(&borrower, &debt_token, &20_000_000i128);

    let stoken_underlying_balance = sut.pool.get_stoken_underlying_balance(&stoken_token);
    let user_balance = debt_config.token.balance(&borrower);
    let treasury_balance = debt_config.token.balance(&treasury_address);
    let user_debt_balance = debt_config.debt_token.balance(&borrower);

    assert_eq!(stoken_underlying_balance, 79_998_543);
    assert_eq!(user_balance, 1_020_000_000);
    assert_eq!(treasury_balance, 1_457);
    assert_eq!(user_debt_balance, 20_002_275);
}

#[test]
fn should_fully_repay() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, debt_config) = fill_pool(&env, &sut);
    let debt_token = &debt_config.token.address;
    let stoken_token = &debt_config.s_token.address;

    env.ledger().with_mut(|li| li.timestamp = DAY);
    let treasury_address = sut.pool.treasury().clone();

    let stoken_underlying_balance = sut.pool.get_stoken_underlying_balance(&stoken_token);
    let user_balance = debt_config.token.balance(&borrower);
    let treasury_balance = debt_config.token.balance(&treasury_address);
    let user_debt_balance = debt_config.debt_token.balance(&borrower);

    assert_eq!(stoken_underlying_balance, 60_000_000);
    assert_eq!(user_balance, 1_040_000_000);
    assert_eq!(treasury_balance, 0);
    assert_eq!(user_debt_balance, 40_000_000);

    sut.pool.deposit(&borrower, &debt_token, &i128::MAX);

    let stoken_underlying_balance = sut.pool.get_stoken_underlying_balance(&stoken_token);
    let user_balance = debt_config.token.balance(&borrower);
    let treasury_balance = debt_config.token.balance(&treasury_address);
    let user_debt_balance = debt_config.debt_token.balance(&borrower);

    assert_eq!(stoken_underlying_balance, 100_001_637);
    assert_eq!(user_balance, 999_995_452);
    assert_eq!(treasury_balance, 2_911);
    assert_eq!(user_debt_balance, 0);
}

#[test]
fn should_change_user_config() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, debt_config) = fill_pool(&env, &sut);
    let debt_token = &debt_config.token.address;

    sut.pool.deposit(&borrower, &debt_token, &i128::MAX);

    let user_config = sut.pool.user_configuration(&borrower);
    let reserve = sut.pool.get_reserve(&debt_config.token.address).unwrap();

    assert_eq!(user_config.is_borrowing(&env, reserve.get_id()), false);
}

#[test]
fn should_emit_events() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, debt_config) = fill_pool(&env, &sut);
    let debt_token = &debt_config.token.address;

    env.ledger().with_mut(|li| li.timestamp = DAY);

    sut.pool.deposit(&borrower, &debt_token.clone(), &i128::MAX);

    let event = env.events().all().pop_back_unchecked();

    assert_eq!(
        vec![&env, event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "repay"), borrower.clone()).into_val(&env),
                (debt_token, 40_004_548i128).into_val(&env)
            ),
        ]
    );
}
