use crate::{tests::sut::init_pool, *};
use soroban_sdk::{testutils::Events, IntoVal, Symbol};

#[test]
fn should_be_active_when_reserve_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let reserve = sut.pool.get_reserve(&sut.token().address).unwrap();

    assert_eq!(reserve.configuration.is_active, true);
}

#[test]
fn should_emit_events() {
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
