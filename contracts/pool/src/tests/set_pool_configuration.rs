#![cfg(test)]
extern crate std;

use soroban_sdk::testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation};
use soroban_sdk::{vec, IntoVal, Symbol};

use crate::tests::sut::init_pool;
use crate::*;

#[test]
fn should_require_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let pool_config = PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        grace_period: 1,
        user_assets_limit: 4,
        min_collat_amount: 0,
        min_debt_amount: 0,
        liquidation_protocol_fee: 0,
    };

    let set_pool_configuration_owner = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &set_pool_configuration_owner,
        &Permission::SetPoolConfiguration,
    );

    sut.pool
        .set_pool_configuration(&set_pool_configuration_owner, &pool_config);

    assert_eq!(
        env.auths(),
        [(
            set_pool_configuration_owner.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "set_pool_configuration"),
                    vec![
                        &env,
                        set_pool_configuration_owner.into_val(&env),
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
    let pause_info_before = sut.pool.pause_info();

    sut.pool.set_pool_configuration(
        &sut.pool_admin,
        &PoolConfig {
            base_asset_address: sut.reserves[1].token.address.clone(),
            base_asset_decimals: sut.reserves[1].token.decimals(),
            flash_loan_fee: 12,
            initial_health: 111,
            timestamp_window: 11,
            grace_period: 3,
            user_assets_limit: 1,
            min_collat_amount: 123,
            min_debt_amount: 1234,
            liquidation_protocol_fee: 5,
        },
    );

    let pause_info_after = sut.pool.pause_info();
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
    assert_eq!(pool_config_before.grace_period, 1);
    assert_eq!(pool_config_before.min_collat_amount, 0);
    assert_eq!(pool_config_before.min_debt_amount, 0);
    assert_eq!(pool_config_before.liquidation_protocol_fee, 0);

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
    assert_eq!(pool_config_after.grace_period, 3);
    assert_eq!(pool_config_after.min_collat_amount, 123);
    assert_eq!(pool_config_after.min_debt_amount, 1234);
    assert_eq!(pool_config_after.liquidation_protocol_fee, 5);

    assert_eq!(
        pool_config_after.grace_period,
        pause_info_after.grace_period_secs
    );
    assert_eq!(pause_info_before.paused, pause_info_after.paused);
    assert_eq!(pause_info_before.unpaused_at, pause_info_after.unpaused_at);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_no_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetPoolConfiguration);
    let no_perm = Address::generate(&env);
    let permissioned = sut.pool.permissioned(&Permission::SetPoolConfiguration);

    assert!(permissioned.binary_search(&no_perm).is_err());

    sut.pool.set_pool_configuration(
        &no_perm,
        &PoolConfig {
            base_asset_address: sut.reserves[1].token.address.clone(),
            base_asset_decimals: sut.reserves[1].token.decimals(),
            flash_loan_fee: 12,
            initial_health: 111,
            timestamp_window: 11,
            grace_period: 3,
            user_assets_limit: 1,
            min_collat_amount: 0,
            min_debt_amount: 0,
            liquidation_protocol_fee: 0,
        },
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_has_another_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetPoolConfiguration);
    let another_perm = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &another_perm,
        &Permission::ClaimProtocolFee,
    );
    let permissioned = sut.pool.permissioned(&Permission::SetPoolConfiguration);

    assert!(permissioned.binary_search(&another_perm).is_err());

    sut.pool.set_pool_configuration(
        &another_perm,
        &PoolConfig {
            base_asset_address: sut.reserves[1].token.address.clone(),
            base_asset_decimals: sut.reserves[1].token.decimals(),
            flash_loan_fee: 12,
            initial_health: 111,
            timestamp_window: 11,
            grace_period: 3,
            user_assets_limit: 1,
            min_collat_amount: 0,
            min_debt_amount: 0,
            liquidation_protocol_fee: 0,
        },
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_permission_revoked() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetPoolConfiguration);
    let revoked_perm = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &revoked_perm,
        &Permission::SetPoolConfiguration,
    );
    sut.pool.revoke_permission(
        &sut.pool_admin,
        &revoked_perm,
        &Permission::SetPoolConfiguration,
    );
    let permissioned = sut.pool.permissioned(&Permission::SetPoolConfiguration);

    assert!(permissioned.binary_search(&revoked_perm).is_err());

    sut.pool.set_pool_configuration(
        &revoked_perm,
        &PoolConfig {
            base_asset_address: sut.reserves[1].token.address.clone(),
            base_asset_decimals: sut.reserves[1].token.decimals(),
            flash_loan_fee: 12,
            initial_health: 111,
            timestamp_window: 11,
            grace_period: 3,
            user_assets_limit: 1,
            min_collat_amount: 0,
            min_debt_amount: 0,
            liquidation_protocol_fee: 0,
        },
    );
}
// #[test]
// #[should_panic(expected = "HostError: Error(Contract, #5)")]
// fn should_require_non_zero() {
//     let env = Env::default();
//     env.mock_all_auths();

//     let sut = init_pool(&env, false);

//     let grace_period = 0;
//     sut.pool.set_grace_period(&grace_period);
// }

// #[test]
// fn should_set_grace_period() {
//     let env = Env::default();
//     env.mock_all_auths();

//     let sut = init_pool(&env, false);
//     let prev_pause_info = sut.pool.pause_info();

//     let grace_period = 1;
//     sut.pool.set_grace_period(&grace_period);
//     let pause_info = sut.pool.pause_info();

//     assert_eq!(grace_period, pause_info.grace_period_secs);
//     assert_eq!(prev_pause_info.paused, pause_info.paused);
//     assert_eq!(prev_pause_info.unpaused_at, pause_info.unpaused_at);
// }
