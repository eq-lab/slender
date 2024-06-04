#![cfg(test)]
extern crate std;

use crate::{tests::sut::init_pool, *};
use soroban_sdk::{
    testutils::{AuthorizedFunction, AuthorizedInvocation, Ledger as _},
    vec, IntoVal, Symbol,
};
use tests::sut::DAY;

#[test]
fn should_require_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    sut.pool.set_pause(&sut.pool_admin, &true);

    assert_eq!(
        env.auths(),
        [(
            sut.pool_admin.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "set_pause"),
                    vec![&env, sut.pool_admin.into_val(&env), true.into_val(&env)]
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
