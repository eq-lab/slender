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

    let set_reserve_status_owner = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &set_reserve_status_owner,
        &Permission::SetReserveStatus,
    );

    sut.pool
        .set_reserve_status(&set_reserve_status_owner, &asset_address.clone(), &true);

    assert_eq!(
        env.auths(),
        [(
            set_reserve_status_owner.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "set_reserve_status"),
                    (set_reserve_status_owner, asset_address.clone(), true).into_val(&env)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
}

#[test]
fn should_set_reserve_status() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();

    sut.pool
        .set_reserve_status(&sut.pool_admin, &asset_address.clone(), &false);
    let reserve = sut.pool.get_reserve(&asset_address).unwrap();

    assert_eq!(reserve.configuration.is_active, false);

    sut.pool
        .set_reserve_status(&sut.pool_admin, &asset_address.clone(), &true);
    let reserve = sut.pool.get_reserve(&asset_address).unwrap();

    assert_eq!(reserve.configuration.is_active, true);
}

#[test]
fn should_be_active_when_reserve_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let reserve = sut.pool.get_reserve(&sut.token().address).unwrap();

    assert_eq!(reserve.configuration.is_active, true);
}

#[test]
fn should_emit_events() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset = sut.token().address.clone();

    assert_eq!(
        sut.pool.set_reserve_status(&sut.pool_admin, &asset, &true),
        ()
    );

    let events = env.events().all().pop_back_unchecked();

    assert_eq!(
        vec![&env, events],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "reserve_activated"), &asset).into_val(&env),
                ().into_val(&env)
            ),
        ]
    );

    assert_eq!(
        sut.pool.set_reserve_status(&sut.pool_admin, &asset, &false),
        ()
    );

    let events = env.events().all().pop_back_unchecked();

    assert_eq!(
        vec![&env, events],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "reserve_deactivated"), &asset).into_val(&env),
                ().into_val(&env)
            ),
        ]
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_no_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetPause);
    let no_perm = Address::generate(&env);
    let permissioned = sut.pool.permissioned(&Permission::SetPause);

    assert!(permissioned.binary_search(&no_perm).is_err());

    let asset_address = sut.token().address.clone();
    sut.pool
        .set_reserve_status(&no_perm, &asset_address.clone(), &false);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_has_another_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetPause);
    let another_perm = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &another_perm,
        &Permission::ClaimProtocolFee,
    );
    let permissioned = sut.pool.permissioned(&Permission::SetPause);

    assert!(permissioned.binary_search(&another_perm).is_err());

    let asset_address = sut.token().address.clone();
    sut.pool
        .set_reserve_status(&another_perm, &asset_address.clone(), &false);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_permission_revoked() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetPause);
    let revoked_perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &revoked_perm, &Permission::SetPause);
    sut.pool
        .revoke_permission(&sut.pool_admin, &revoked_perm, &Permission::SetPause);
    let permissioned = sut.pool.permissioned(&Permission::SetPause);

    assert!(permissioned.binary_search(&revoked_perm).is_err());

    let asset_address = sut.token().address.clone();
    sut.pool
        .set_reserve_status(&revoked_perm, &asset_address.clone(), &false);
}
