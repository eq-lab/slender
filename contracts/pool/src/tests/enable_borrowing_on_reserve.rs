#![cfg(test)]
extern crate std;

use crate::{tests::sut::init_pool, *};
use soroban_sdk::{
    testutils::{AuthorizedFunction, AuthorizedInvocation, Events},
    vec, IntoVal, Symbol,
};

#[test]
fn should_require_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();

    sut.pool
        .enable_borrowing_on_reserve(&asset_address.clone(), &true);

    assert_eq!(
        env.auths(),
        [(
            sut.pool_admin,
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "enable_borrowing_on_reserve"),
                    (asset_address.clone(), true).into_val(&env)
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
        .enable_borrowing_on_reserve(&asset_address.clone(), &false);
    let reserve = sut.pool.get_reserve(&asset_address).unwrap();

    assert_eq!(reserve.configuration.borrowing_enabled, false);

    sut.pool
        .enable_borrowing_on_reserve(&asset_address.clone(), &true);
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

    assert_eq!(sut.pool.enable_borrowing_on_reserve(&asset, &true), ());

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

    assert_eq!(sut.pool.enable_borrowing_on_reserve(&asset, &false), ());

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
        .enable_borrowing_on_reserve(&rwa_address.clone(), &false);
    let reserve = sut.pool.get_reserve(&rwa_address).unwrap();

    assert_eq!(reserve.configuration.borrowing_enabled, false);

    sut.pool
        .enable_borrowing_on_reserve(&rwa_address.clone(), &true);
}
