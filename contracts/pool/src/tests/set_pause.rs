#![cfg(test)]
extern crate std;

use crate::tests::sut::init_pool;
use crate::*;
use soroban_sdk::testutils::{AuthorizedFunction, AuthorizedInvocation};
use soroban_sdk::{vec, IntoVal, Symbol};
use tests::sut::{set_time, DAY};

#[test]
fn should_require_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    sut.pool.set_pause(&true);

    assert_eq!(
        env.auths(),
        [(
            sut.pool_admin,
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "set_pause"),
                    vec![&env, true.into_val(&env)]
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

    sut.pool.set_pause(&true);
    let pause_info = sut.pool.pause_info();
    assert!(pause_info.paused);
    assert_eq!(
        prev_pause_info.grace_period_secs,
        pause_info.grace_period_secs
    );
    assert_eq!(prev_pause_info.unpaused_at, pause_info.unpaused_at);

    // change current time and check unpaused_at
    set_time(&env, &sut, 2 * DAY, false);
    let expected_unpaused_at = env.ledger().timestamp();
    sut.pool.set_pause(&false);
    let pause_info = sut.pool.pause_info();
    assert!(!pause_info.paused);
    assert_eq!(
        prev_pause_info.grace_period_secs,
        pause_info.grace_period_secs
    );
    assert_eq!(expected_unpaused_at, pause_info.unpaused_at);
    assert!(prev_pause_info.unpaused_at < pause_info.unpaused_at);
}
