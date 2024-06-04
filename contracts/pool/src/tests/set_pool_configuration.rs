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

    let pool_config = PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        user_assets_limit: 4,
        min_collat_amount: 0,
        min_debt_amount: 0,
        liquidation_protocol_fee: 0,
    };

    sut.pool
        .set_pool_configuration(&sut.pool_admin, &pool_config);

    assert_eq!(
        env.auths(),
        [(
            sut.pool_admin.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "set_pool_configuration"),
                    vec![
                        &env,
                        sut.pool_admin.into_val(&env),
                        pool_config.into_val(&env)
                    ]
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
}

#[test]
fn should_set_pool_configuration() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let pool_config_before = sut.pool.pool_configuration();

    sut.pool.set_pool_configuration(
        &sut.pool_admin,
        &PoolConfig {
            base_asset_address: sut.reserves[1].token.address.clone(),
            base_asset_decimals: sut.reserves[1].token.decimals(),
            flash_loan_fee: 12,
            initial_health: 111,
            timestamp_window: 11,
            user_assets_limit: 1,
            min_collat_amount: 0,
            min_debt_amount: 0,
            liquidation_protocol_fee: 0,
        },
    );

    let pool_config_after = sut.pool.pool_configuration();

    assert_eq!(
        pool_config_before.base_asset_address,
        sut.reserves[0].token.address
    );
    assert_eq!(
        pool_config_before.base_asset_decimals,
        sut.reserves[0].token.decimals()
    );
    assert_eq!(pool_config_before.flash_loan_fee, 5);
    assert_eq!(pool_config_before.initial_health, 0);
    assert_eq!(pool_config_before.timestamp_window, 20);
    assert_eq!(pool_config_before.user_assets_limit, 4);

    assert_eq!(
        pool_config_after.base_asset_address,
        sut.reserves[1].token.address
    );
    assert_eq!(
        pool_config_after.base_asset_decimals,
        sut.reserves[1].token.decimals()
    );
    assert_eq!(pool_config_after.flash_loan_fee, 12);
    assert_eq!(pool_config_after.initial_health, 111);
    assert_eq!(pool_config_after.timestamp_window, 11);
    assert_eq!(pool_config_after.user_assets_limit, 1);
}
