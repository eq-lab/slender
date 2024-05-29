#![cfg(test)]
extern crate std;

use soroban_sdk::{
    testutils::{AuthorizedFunction, AuthorizedInvocation},
    vec, IntoVal, Symbol,
};

use crate::{tests::sut::init_pool, *};

#[test]
fn should_require_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let grace_period = 1;
    sut.pool.set_grace_period(&grace_period);

    assert_eq!(
        env.auths(),
        [(
            sut.pool_admin,
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "set_grace_period"),
                    vec![&env, grace_period.into_val(&env)]
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
    sut.pool.set_grace_period(&grace_period);
}

#[test]
fn should_set_grace_period() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let prev_pause_info = sut.pool.pause_info();

    let grace_period = 1;
    sut.pool.set_grace_period(&grace_period);
    let pause_info = sut.pool.pause_info();

    assert_eq!(grace_period, pause_info.grace_period_secs);
    assert_eq!(prev_pause_info.paused, pause_info.paused);
    assert_eq!(prev_pause_info.unpaused_at, pause_info.unpaused_at);
}
