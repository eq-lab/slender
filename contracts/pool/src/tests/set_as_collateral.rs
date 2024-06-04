use super::sut::{init_pool, Sut};
use crate::*;
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::token::StellarAssetClient as TokenAdminClient;
use soroban_sdk::{vec, IntoVal, Symbol};

#[test]
fn should_enable_collateral_when_no_debt() {
    let env = Env::default();
    env.mock_all_auths();
    let (sut, user, reserve_index, token) = init(&env);

    assert!(sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, reserve_index));

    assert_eq!(sut.pool.set_as_collateral(&user, &token, &true), ());

    assert!(sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, reserve_index));

    assert_eq!(sut.pool.set_as_collateral(&user, &token, &false), ());

    assert!(!sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, reserve_index));

    assert_eq!(sut.pool.set_as_collateral(&user, &token, &true), ());

    assert!(sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, reserve_index));
}

#[test]
fn should_disable_collateral_when_deposited() {
    let env = Env::default();
    env.mock_all_auths();
    let (sut, user, (collat_reserve_index, debt_reserve_index), (collat_token, _)) =
        init_with_debt(&env);
    deposit(&sut.pool, &sut.reserves[2].token_admin, &user);
    deposit(&sut.pool, &sut.reserves[2].token_admin, &user);

    assert!(sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, collat_reserve_index));

    assert_eq!(sut.pool.set_as_collateral(&user, &collat_token, &false), ());

    assert!(!sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, collat_reserve_index));

    assert!(!sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, debt_reserve_index));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #204)")]
fn should_fail_when_has_debt() {
    let env = Env::default();
    env.mock_all_auths();

    let (sut, user, (_, _), (_, debt_token)) = init_with_debt(&env);
    deposit(&sut.pool, &sut.reserves[2].token_admin, &user);

    sut.pool.set_as_collateral(&user, &debt_token, &true);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #301)")]
fn should_fail_when_npv_fails_bellow_initial_health() {
    let env = Env::default();
    env.mock_all_auths();

    let (sut, user, (collat_reserve_index, _), (collat_token, _)) = init_with_debt(&env);

    sut.pool.set_pool_configuration(
        &sut.pool_admin,
        &PoolConfig {
            base_asset_address: sut.reserves[0].token.address.clone(),
            base_asset_decimals: sut.reserves[0].token.decimals(),
            flash_loan_fee: 5,
            initial_health: 2_500,
            timestamp_window: 20,
            user_assets_limit: 4,
            min_collat_amount: 0,
            min_debt_amount: 0,
            liquidation_protocol_fee: 0,
        },
    );

    sut.pool
        .set_as_collateral(&user, &collat_token.clone(), &false);

    assert!(sut
        .pool
        .user_configuration(&user)
        .is_using_as_collateral(&env, collat_reserve_index));
}

#[test]
fn should_emit_events() {
    let env = Env::default();
    env.mock_all_auths();
    let (sut, user, _, token) = init(&env);

    assert_eq!(sut.pool.set_as_collateral(&user, &token, &false), ());

    let coll_disabled_event = env.events().all().pop_back_unchecked();

    assert_eq!(
        vec![&env, coll_disabled_event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "reserve_used_as_coll_disabled"), &user).into_val(&env),
                token.into_val(&env)
            ),
        ]
    );

    assert_eq!(sut.pool.set_as_collateral(&user, &token, &true), ());

    let coll_disabled_event = env.events().all().pop_back_unchecked();

    assert_eq!(
        vec![&env, coll_disabled_event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "reserve_used_as_coll_enabled"), &user).into_val(&env),
                token.into_val(&env)
            ),
        ]
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #205)")]
fn rwa_fail_when_exceed_assets_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let (sut, user, (_, _), (_, _)) = init_with_debt(&env);
    deposit(&sut.pool, &sut.reserves[0].token_admin, &user);
    deposit(&sut.pool, &sut.reserves[2].token_admin, &user);

    assert_eq!(
        sut.pool
            .set_as_collateral(&user, &&sut.reserves[0].token.address, &false),
        ()
    );

    sut.pool.set_pool_configuration(
        &sut.pool_admin,
        &PoolConfig {
            base_asset_address: sut.reserves[0].token.address.clone(),
            base_asset_decimals: sut.reserves[0].token.decimals(),
            flash_loan_fee: 5,
            initial_health: 0,
            timestamp_window: 20,
            user_assets_limit: 2,
            min_collat_amount: 0,
            min_debt_amount: 300_000,
            liquidation_protocol_fee: 0,
        },
    );

    sut.pool
        .set_as_collateral(&user, &&sut.reserves[0].token.address, &true);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #207)")]
fn should_fail_when_collat_lt_min_position_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let (sut, user, (_, _), (collat_token, _)) = init_with_debt(&env);
    deposit(&sut.pool, &sut.reserves[0].token_admin, &user);
    deposit(&sut.pool, &sut.reserves[2].token_admin, &user);

    sut.pool.set_pool_configuration(
        &sut.pool_admin,
        &PoolConfig {
            base_asset_address: sut.reserves[0].token.address.clone(),
            base_asset_decimals: sut.reserves[0].token.decimals(),
            flash_loan_fee: 5,
            initial_health: 0,
            timestamp_window: 20,
            user_assets_limit: 2,
            min_collat_amount: 7_000_000,
            min_debt_amount: 0,
            liquidation_protocol_fee: 0,
        },
    );

    assert_eq!(sut.pool.set_as_collateral(&user, &collat_token, &false), ());
}

/// Init for set_as_collateral tests.
/// Returns Sut, user address, reserve index and token address
fn init(env: &Env) -> (Sut, Address, u8, Address) {
    let sut = init_pool(env, false);

    let user = Address::generate(env);
    deposit(&sut.pool, sut.token_admin(), &user);
    let reserve_index = sut
        .pool
        .get_reserve(&sut.token().address)
        .expect("reserve")
        .get_id();
    let address = sut.token().address.clone();
    (sut, user, reserve_index, address)
}

/// Returns Sut, user address, collat reserve index, debt reserve index, collat token address, debt token address
pub fn init_with_debt(env: &Env) -> (Sut, Address, (u8, u8), (Address, Address)) {
    let (sut, user, collat_reserve_index, collat_address) = init(env);
    let lender = Address::generate(env);
    let token_admin = &sut.reserves[1].token_admin;

    deposit(&sut.pool, token_admin, &lender);
    sut.pool.borrow(&user, &token_admin.address, &600_000_000);

    let debt_reserve_index = sut
        .pool
        .get_reserve(&token_admin.address)
        .expect("reserve")
        .get_id();

    let debt_address = token_admin.address.clone();
    (
        sut,
        user,
        (collat_reserve_index, debt_reserve_index),
        (collat_address, debt_address),
    )
}

fn deposit(pool: &LendingPoolClient, token: &TokenAdminClient, user: &Address) {
    token.mint(&user, &1_000_000_000);
    pool.deposit(&user, &token.address, &1_000_000_000);
}
