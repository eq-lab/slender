use crate::*;
use soroban_sdk::{testutils::Address as _, token::AdminClient as TokenAdminClient};

use super::sut::{init_pool, Sut};
extern crate std;

/// Init for set_as_collateral tests.
/// Returns Sut, user address, reserve index and token address
fn init(env: &Env) -> (Sut, Address, u8, Address) {
    let sut = init_pool(env);
    //TODO: optimize gas
    env.budget().reset_unlimited();
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
fn init_with_debt(env: &Env) -> (Sut, Address, (u8, u8), (Address, Address)) {
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

#[test]
fn set_as_collateral_no_debt() {
    let env = Env::default();
    env.mock_all_auths();
    let (sut, user, reserve_index, token) = init(&env);

    assert!(sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, reserve_index));

    assert_eq!(
        sut.pool
            .set_as_collateral(&user, &sut.token().address, &true),
        ()
    );

    assert!(sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, reserve_index));

    assert_eq!(sut.pool.set_as_collateral(&user, &token, &false), ());

    assert!(!sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, reserve_index));

    assert_eq!(
        sut.pool
            .set_as_collateral(&user, &sut.token().address, &true),
        ()
    );

    assert!(sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, reserve_index));
}

#[test]
fn set_as_collateral_false_with_debt() {
    let env = Env::default();
    env.mock_all_auths();
    let (sut, user, (collat_reserve_index, debt_reserve_index), (collat_token, u_debt_token)) =
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

    assert_eq!(sut.pool.set_as_collateral(&user, &u_debt_token, &false), ());

    assert!(!sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, debt_reserve_index));
}

#[test]
fn set_as_collateral_true_with_debt() {
    let env = Env::default();
    env.mock_all_auths();

    let env = Env::default();
    env.mock_all_auths();
    let (sut, user, (collat_reserve_index, debt_reserve_index), (collat_token, _u_debt_token)) =
        init_with_debt(&env);
    deposit(&sut.pool, &sut.reserves[2].token_admin, &user);

    assert!(sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, collat_reserve_index));

    assert_eq!(sut.pool.set_as_collateral(&user, &collat_token, &true), ());

    assert!(sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, collat_reserve_index));

    assert!(!sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, debt_reserve_index));

    // TODO: after soroban fix
    // assert_eq!(sut.pool.try_set_as_collateral(&user, &_u_debt_token, &true).unwrap_err().unwrap(), Error::MustNotHaveDebt);

    // assert!(!sut
    //     .pool
    //     .user_configuration(&user)
    //     .is_using_as_collateral(&env, debt_reserve_index));
}

#[test]
fn set_as_collateral_bad_position() {
    let env = Env::default();
    env.mock_all_auths();
    let (sut, user, (collat_reserve_index, _), (_collat_token, _)) = init_with_debt(&env);

    assert!(sut.pool.account_position(&user).npv == 0, "configuration");

    assert!(sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, collat_reserve_index));

    // TODO: after soroban fix
    // assert_eq!(
    //     sut.pool
    //         .try_set_as_collateral(&user, &_collat_token, &false)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::BadPosition
    // );

    // assert!(sut
    //     .pool
    //     .user_configuration(&user)
    //     .is_using_as_collateral(&env, collat_reserve_index));
}
