#![cfg(test)]
extern crate std;

use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
    vec, IntoVal, Symbol,
};

use crate::{tests::sut::init_pool, *};

#[test]
fn should_require_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let grace_period = 1;

    let set_grace_period_owner = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &set_grace_period_owner,
        &Permission::SetGracePeriod,
    );

    sut.pool
        .set_grace_period(&set_grace_period_owner, &grace_period);

    assert_eq!(
        env.auths(),
        [(
            set_grace_period_owner.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "set_grace_period"),
                    vec![
                        &env,
                        set_grace_period_owner.into_val(&env),
                        grace_period.into_val(&env)
                    ]
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")]
fn should_require_non_zero() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let grace_period = 0;
    sut.pool.set_grace_period(&sut.pool_admin, &grace_period);
}

#[test]
fn should_set_grace_period() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let prev_pause_info = sut.pool.pause_info();

    let grace_period = 1;
    sut.pool.set_grace_period(&sut.pool_admin, &grace_period);
    let pause_info = sut.pool.pause_info();

    assert_eq!(grace_period, pause_info.grace_period_secs);
    assert_eq!(prev_pause_info.paused, pause_info.paused);
    assert_eq!(prev_pause_info.unpaused_at, pause_info.unpaused_at);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_no_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetGracePeriod);
    let no_perm = Address::generate(&env);
    let permissioned = sut.pool.permissioned(&Permission::SetGracePeriod);

    assert!(permissioned.binary_search(&no_perm).is_err());

    sut.pool.set_grace_period(&no_perm, &1);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_has_another_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetGracePeriod);
    let another_perm = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &another_perm,
        &Permission::ClaimProtocolFee,
    );
    let permissioned = sut.pool.permissioned(&Permission::SetGracePeriod);

    assert!(permissioned.binary_search(&another_perm).is_err());

    sut.pool.set_grace_period(&another_perm, &1);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_permission_revoked() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetGracePeriod);
    let revoked_perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &revoked_perm, &Permission::SetGracePeriod);
    sut.pool
        .revoke_permission(&sut.pool_admin, &revoked_perm, &Permission::SetGracePeriod);
    let permissioned = sut.pool.permissioned(&Permission::SetGracePeriod);

    assert!(permissioned.binary_search(&revoked_perm).is_err());

    sut.pool.set_grace_period(&revoked_perm, &1);
}
