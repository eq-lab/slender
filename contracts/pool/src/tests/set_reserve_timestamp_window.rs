#![cfg(test)]
extern crate std;

use soroban_sdk::testutils::{AuthorizedFunction, AuthorizedInvocation};
use soroban_sdk::{vec, IntoVal, Symbol};

use crate::tests::sut::init_pool;
use crate::*;

#[test]
fn should_require_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    sut.pool.set_reserve_timestamp_window(&1);

    assert_eq!(
        env.auths(),
        [(
            sut.pool_admin.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "set_reserve_timestamp_window"),
                    vec![&env, 1u64.into_val(&env)]
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
}

#[test]
fn should_set_reserve_timestamp_window() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let window_initial = sut.pool.reserve_timestamp_window();

    sut.pool.set_reserve_timestamp_window(&123);
    let window_after = sut.pool.reserve_timestamp_window();

    assert_eq!(window_initial, 20);
    assert_eq!(window_after, 123);
}
