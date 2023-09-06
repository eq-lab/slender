use super::sut::{init_pool, Sut};
use crate::*;
use soroban_sdk::{
    testutils::{Address as _, Events},
    token::AdminClient as TokenAdminClient,
    vec, IntoVal, Symbol,
};

#[test]
fn should_enable_collateral_when_no_debt() {
    let env = Env::default();
    env.mock_all_auths();
    let (sut, user, reserve_index, token) = init(&env);

    assert!(sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, reserve_index));

    assert_eq!(sut.pool.set_as_collateral(&user, &token, &true), ());

    assert!(sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, reserve_index));

    assert_eq!(sut.pool.set_as_collateral(&user, &token, &false), ());

    assert!(!sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, reserve_index));

    assert_eq!(sut.pool.set_as_collateral(&user, &token, &true), ());

    assert!(sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, reserve_index));
}

#[test]
fn should_disable_collateral_when_deposited() {
    let env = Env::default();
    env.mock_all_auths();
    let (sut, user, (collat_reserve_index, debt_reserve_index), (collat_token, _)) =
        init_with_debt(&env);
    deposit(&sut.pool, &sut.reserves[2].token_admin, &user);
    deposit(&sut.pool, &sut.reserves[2].token_admin, &user);

    assert!(sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, collat_reserve_index));

    assert_eq!(sut.pool.set_as_collateral(&user, &collat_token, &false), ());

    assert!(!sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, collat_reserve_index));

    assert!(!sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, debt_reserve_index));
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_has_debt() {
    let env = Env::default();
    env.mock_all_auths();

    let (sut, user, (_, _), (_, debt_token)) = init_with_debt(&env);
    deposit(&sut.pool, &sut.reserves[2].token_admin, &user);

    sut.pool.set_as_collateral(&user, &debt_token, &true);

    // assert_eq!(sut.pool.try_set_as_collateral(&user, &_u_debt_token, &true).unwrap_err().unwrap(), Error::MustNotHaveDebt);
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_bad_position() {
    let env = Env::default();
    env.mock_all_auths();

    let (sut, user, (collat_reserve_index, _), (collat_token, _)) = init_with_debt(&env);

    sut.pool
        .set_as_collateral(&user, &collat_token.clone(), &false);

    assert!(sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, collat_reserve_index));

    // assert_eq!(
    //     sut.pool
    //         .try_set_as_collateral(&user, &collat_token.clone(), &false)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::BadPosition
    // );
}

#[test]
fn should_emit_events() {
    let env = Env::default();
    env.mock_all_auths();
    let (sut, user, _, token) = init(&env);

    assert_eq!(sut.pool.set_as_collateral(&user, &token, &false), ());

    let coll_disabled_event = env.events().all().pop_back_unchecked();

    assert_eq!(
        vec![&env, coll_disabled_event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "reserve_used_as_coll_disabled"), &user).into_val(&env),
                token.into_val(&env)
            ),
        ]
    );

    assert_eq!(sut.pool.set_as_collateral(&user, &token, &true), ());

    let coll_disabled_event = env.events().all().pop_back_unchecked();

    assert_eq!(
        vec![&env, coll_disabled_event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "reserve_used_as_coll_enabled"), &user).into_val(&env),
                token.into_val(&env)
            ),
        ]
    );
}

/// Init for set_as_collateral tests.
/// Returns Sut, user address, reserve index and token address
fn init(env: &Env) -> (Sut, Address, u8, Address) {
    let sut = init_pool(env, false);

    let user = Address::random(env);
    deposit(&sut.pool, sut.token_admin(), &user);
    let reserve_index = sut
        .pool
        .get_reserve(&sut.token().address)
        .expect("reserve")
        .get_id();
    let address = sut.token().address.clone();
    (sut, user, reserve_index, address)
}

/// Returns Sut, user address, collat reserve index, debt reserve index, collat token address, debt token address
pub fn init_with_debt(env: &Env) -> (Sut, Address, (u8, u8), (Address, Address)) {
    let (sut, user, collat_reserve_index, collat_address) = init(env);
    let lender = Address::random(env);
    let token_admin = &sut.reserves[1].token_admin;

    deposit(&sut.pool, token_admin, &lender);
    sut.pool.borrow(&user, &token_admin.address, &600_000_000);

    let debt_reserve_index = sut
        .pool
        .get_reserve(&token_admin.address)
        .expect("reserve")
        .get_id();

    let debt_address = token_admin.address.clone();
    (
        sut,
        user,
        (collat_reserve_index, debt_reserve_index),
        (collat_address, debt_address),
    )
}

fn deposit(pool: &LendingPoolClient, token: &TokenAdminClient, user: &Address) {
    token.mint(&user, &1_000_000_000);
    pool.deposit(&user, &token.address, &1_000_000_000);
}
