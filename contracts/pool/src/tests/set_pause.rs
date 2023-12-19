#![cfg(test)]
extern crate std;

use crate::{tests::sut::init_pool, *};
use soroban_sdk::{
    testutils::{AuthorizedFunction, AuthorizedInvocation},
    vec, IntoVal, Symbol,
};

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

    sut.pool.set_pause(&true);
    assert!(sut.pool.paused());

    sut.pool.set_pause(&false);
    assert!(!sut.pool.paused());
}
