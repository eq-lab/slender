#![cfg(test)]
extern crate std;

use crate::{tests::sut::init_pool, *};
use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Events},
    vec, IntoVal, Symbol,
};

#[test]
fn should_require_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();

    let set_borrowing_owner = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &set_borrowing_owner,
        &Permission::SetReserveBorrowing,
    );

    sut.pool
        .enable_borrowing_on_reserve(&set_borrowing_owner, &asset_address.clone(), &true);

    assert_eq!(
        env.auths(),
        [(
            set_borrowing_owner.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "enable_borrowing_on_reserve"),
                    (set_borrowing_owner, asset_address.clone(), true).into_val(&env)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
}

#[test]
fn should_set_borrowing_status() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();

    sut.pool
        .enable_borrowing_on_reserve(&sut.pool_admin, &asset_address.clone(), &false);
    let reserve = sut.pool.get_reserve(&asset_address).unwrap();

    assert_eq!(reserve.configuration.borrowing_enabled, false);

    sut.pool
        .enable_borrowing_on_reserve(&sut.pool_admin, &asset_address.clone(), &true);
    let reserve = sut.pool.get_reserve(&asset_address).unwrap();

    assert_eq!(reserve.configuration.borrowing_enabled, true);
}

#[test]
fn should_be_active_when_reserve_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let reserve = sut.pool.get_reserve(&sut.token().address).unwrap();

    assert_eq!(reserve.configuration.borrowing_enabled, true);
}

#[test]
fn should_emit_events() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset = sut.token().address.clone();

    assert_eq!(
        sut.pool
            .enable_borrowing_on_reserve(&sut.pool_admin, &asset, &true),
        ()
    );

    let events = env.events().all().pop_back_unchecked();

    assert_eq!(
        vec![&env, events],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "borrowing_enabled"), &asset).into_val(&env),
                ().into_val(&env)
            ),
        ]
    );

    assert_eq!(
        sut.pool
            .enable_borrowing_on_reserve(&sut.pool_admin, &asset, &false),
        ()
    );

    let events = env.events().all().pop_back_unchecked();

    assert_eq!(
        vec![&env, events],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "borrowing_disabled"), &asset).into_val(&env),
                ().into_val(&env)
            ),
        ]
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #110)")]
fn should_fail_when_enable_rwa() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let rwa_address = sut.rwa_config().token.address.clone();

    sut.pool
        .enable_borrowing_on_reserve(&sut.pool_admin, &rwa_address.clone(), &false);
    let reserve = sut.pool.get_reserve(&rwa_address).unwrap();

    assert_eq!(reserve.configuration.borrowing_enabled, false);

    sut.pool
        .enable_borrowing_on_reserve(&sut.pool_admin, &rwa_address.clone(), &true);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_no_permission_false() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetReserveBorrowing);
    let no_perm = Address::generate(&env);
    let permissioned = sut.pool.permissioned(&Permission::SetReserveBorrowing);

    assert!(permissioned.binary_search(&no_perm).is_err());

    sut.pool
        .enable_borrowing_on_reserve(&no_perm, &asset_address.clone(), &false);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_no_permission_true() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetReserveBorrowing);
    let no_perm = Address::generate(&env);
    let permissioned = sut.pool.permissioned(&Permission::SetReserveBorrowing);

    assert!(permissioned.binary_search(&no_perm).is_err());

    sut.pool
        .enable_borrowing_on_reserve(&no_perm, &asset_address.clone(), &true);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_has_another_permission_false() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetReserveBorrowing);
    let another_perm = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &another_perm,
        &Permission::ClaimProtocolFee,
    );

    let permissioned = sut.pool.permissioned(&Permission::SetReserveBorrowing);
    assert!(permissioned.binary_search(&another_perm).is_err());

    sut.pool
        .enable_borrowing_on_reserve(&another_perm, &asset_address.clone(), &false);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_has_another_permission_true() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetReserveBorrowing);
    let another_perm = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &another_perm,
        &Permission::ClaimProtocolFee,
    );

    let permissioned = sut.pool.permissioned(&Permission::SetReserveBorrowing);
    assert!(permissioned.binary_search(&another_perm).is_err());

    sut.pool
        .enable_borrowing_on_reserve(&another_perm, &asset_address.clone(), &true);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_permission_revoked_false() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetReserveBorrowing);

    let revoked_perm = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &revoked_perm,
        &Permission::SetReserveBorrowing,
    );
    sut.pool.revoke_permission(
        &sut.pool_admin,
        &revoked_perm,
        &Permission::SetReserveBorrowing,
    );

    let permissioned = sut.pool.permissioned(&Permission::SetReserveBorrowing);
    assert!(permissioned.binary_search(&revoked_perm).is_err());

    sut.pool
        .enable_borrowing_on_reserve(&revoked_perm, &asset_address.clone(), &false);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_permission_revoked_true() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetReserveBorrowing);

    let revoked_perm = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &revoked_perm,
        &Permission::SetReserveBorrowing,
    );
    sut.pool.revoke_permission(
        &sut.pool_admin,
        &revoked_perm,
        &Permission::SetReserveBorrowing,
    );

    let permissioned = sut.pool.permissioned(&Permission::SetReserveBorrowing);
    assert!(permissioned.binary_search(&revoked_perm).is_err());

    sut.pool
        .enable_borrowing_on_reserve(&revoked_perm, &asset_address.clone(), &true);
}
