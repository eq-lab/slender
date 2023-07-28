use crate::tests::pool_test::init_pool;
use crate::*;
use soroban_sdk::{testutils::Events, IntoVal, Symbol};

extern crate std;

#[test]
fn set_reserve_status_activated_by_default() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let reserve = sut.pool.get_reserve(&sut.token().address).unwrap();

    assert_eq!(reserve.configuration.is_active, true);
}

#[test]
fn events() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let asset = sut.token().address.clone();

    assert_eq!(sut.pool.set_reserve_status(&asset, &true), ());

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

    assert_eq!(sut.pool.set_reserve_status(&asset, &false), ());

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
