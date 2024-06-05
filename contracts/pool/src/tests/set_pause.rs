#![cfg(test)]
extern crate std;

use crate::{tests::sut::init_pool, *};
use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Ledger as _},
    vec, IntoVal, Symbol,
};
use tests::sut::DAY;

#[test]
fn should_require_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let set_pause_owner = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &set_pause_owner, &Permission::SetPause);

    sut.pool.set_pause(&set_pause_owner, &true);

    assert_eq!(
        env.auths(),
        [(
            set_pause_owner.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "set_pause"),
                    vec![&env, set_pause_owner.into_val(&env), true.into_val(&env)]
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
}

#[test]
fn should_set_pause() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let prev_pause_info = sut.pool.pause_info();

    sut.pool.set_pause(&sut.pool_admin, &true);
    let pause_info = sut.pool.pause_info();
    assert!(pause_info.paused);
    assert_eq!(
        prev_pause_info.grace_period_secs,
        pause_info.grace_period_secs
    );
    assert_eq!(prev_pause_info.unpaused_at, pause_info.unpaused_at);

    // change current time and check unpaused_at
    env.ledger().with_mut(|li| li.timestamp = 2 * DAY);
    let expected_unpaused_at = env.ledger().timestamp();
    sut.pool.set_pause(&sut.pool_admin, &false);
    let pause_info = sut.pool.pause_info();
    assert!(!pause_info.paused);
    assert_eq!(
        prev_pause_info.grace_period_secs,
        pause_info.grace_period_secs
    );
    assert_eq!(expected_unpaused_at, pause_info.unpaused_at);
    assert!(prev_pause_info.unpaused_at < pause_info.unpaused_at);
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

    sut.pool.set_pause(&no_perm, &false);
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

    sut.pool.set_pause(&another_perm, &true);
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

    sut.pool.set_pause(&revoked_perm, &true);
}
