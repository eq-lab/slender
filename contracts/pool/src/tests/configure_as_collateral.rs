#![cfg(test)]
extern crate std;

use crate::{tests::sut::init_pool, *};
use soroban_sdk::{
    testutils::{AuthorizedFunction, AuthorizedInvocation, Events},
    vec, IntoVal, Symbol,
};

#[test]
fn shoould_require_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();
    let decimals = sut.s_token().decimals();
    let params = CollateralParamsInput {
        liq_bonus: 11_000,
        liq_cap: 100_000_000 * 10_i128.pow(decimals),
        util_cap: 9_000,
        discount: 6_000,
    };

    sut.pool
        .configure_as_collateral(&asset_address.clone(), &params.clone());

    assert_eq!(
        env.auths(),
        [(
            sut.pool_admin,
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "configure_as_collateral"),
                    (asset_address.clone(), params).into_val(&env)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #401)")]
fn shoould_fail_when_invalid_discount() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();
    let decimals = sut.s_token().decimals();
    let params = CollateralParamsInput {
        liq_bonus: 11_000,
        liq_cap: 100_000_000 * 10_i128.pow(decimals),
        util_cap: 9_000,
        discount: 10_001,
    };

    sut.pool
        .configure_as_collateral(&asset_address.clone(), &params.clone());
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #401)")]
fn shoould_fail_when_invalid_util_cap() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();
    let decimals = sut.s_token().decimals();
    let params = CollateralParamsInput {
        liq_bonus: 11_000,
        liq_cap: 100_000_000 * 10_i128.pow(decimals),
        util_cap: 10_001,
        discount: 6_000,
    };

    sut.pool
        .configure_as_collateral(&asset_address.clone(), &params.clone());
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #403)")]
fn shoould_fail_when_invalid_liq_bonus() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();
    let decimals = sut.s_token().decimals();
    let params = CollateralParamsInput {
        liq_bonus: 9_000,
        liq_cap: 100_000_000 * 10_i128.pow(decimals),
        util_cap: 10_000,
        discount: 6_000,
    };

    sut.pool
        .configure_as_collateral(&asset_address.clone(), &params.clone());
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #404)")]
fn shoould_fail_when_invalid_liq_cap() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();
    let params = CollateralParamsInput {
        liq_bonus: 11_000,
        liq_cap: -1,
        util_cap: 10_000,
        discount: 6_000,
    };

    sut.pool
        .configure_as_collateral(&asset_address.clone(), &params.clone());
}

#[test]
fn shoould_set_collateral_config() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();
    let decimals = sut.s_token().decimals();
    let params = CollateralParamsInput {
        liq_bonus: 12_000,
        liq_cap: 200_000_000 * 10_i128.pow(decimals),
        util_cap: 8_000,
        discount: 5_000,
    };

    sut.pool
        .configure_as_collateral(&asset_address.clone(), &params.clone());

    let reserve = sut.pool.get_reserve(&asset_address).unwrap();

    assert_eq!(reserve.configuration.discount, params.discount);
    assert_eq!(reserve.configuration.liq_cap, params.liq_cap);
    assert_eq!(reserve.configuration.util_cap, params.util_cap);
    assert_eq!(reserve.configuration.discount, params.discount);
}

#[test]
fn should_emit_events() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let asset_address = sut.token().address.clone();
    let decimals = sut.s_token().decimals();
    let params = CollateralParamsInput {
        liq_bonus: 11_000,
        liq_cap: 100_000_000 * 10_i128.pow(decimals),
        util_cap: 9_000,
        discount: 6_000,
    };

    assert_eq!(
        sut.pool
            .configure_as_collateral(&asset_address.clone(), &params.clone()),
        ()
    );

    let events = env.events().all().pop_back_unchecked();

    assert_eq!(
        vec![&env, events],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "collat_config_change"), &asset_address).into_val(&env),
                (
                    params.liq_bonus,
                    params.liq_cap,
                    params.util_cap,
                    params.discount
                )
                    .into_val(&env)
            ),
        ]
    );
}
