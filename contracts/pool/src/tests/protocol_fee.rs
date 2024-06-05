#![cfg(test)]
extern crate std;

use pool_interface::types::{permission::Permission, pool_config::PoolConfig};
use price_feed_interface::types::{asset::Asset, price_data::PriceData};
use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Ledger},
    vec, Address, Env, IntoVal, Symbol,
};

use crate::tests::sut::fill_pool_six;

use super::sut::{create_token_contract, fill_pool, init_pool, Sut, DAY};

fn generate_protocol_fee(env: &Env, sut: &Sut, debt_token: &Address, borrower: &Address) -> i128 {
    env.ledger().with_mut(|li| li.timestamp = 2 * DAY);

    let protocol_fee_before = sut.pool.protocol_fee(debt_token);

    sut.pool.repay(&borrower, debt_token, &i128::MAX);

    let protocol_fee_after = sut.pool.protocol_fee(debt_token);

    protocol_fee_after - protocol_fee_before
}

#[test]
fn should_read_protocol_fee() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, true);
    let expected_fee = generate_protocol_fee(&env, &sut, &debt_config.token.address, &borrower);
    let actual_fee = sut.pool.protocol_fee(&debt_config.token.address);

    assert_eq!(expected_fee, actual_fee);
}

#[test]
fn should_require_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, true);
    let _ = generate_protocol_fee(&env, &sut, &debt_config.token.address, &borrower);
    let recipient = Address::generate(&env);

    let protocol_fee_owner = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &protocol_fee_owner,
        &Permission::ClaimProtocolFee,
    );

    sut.pool
        .claim_protocol_fee(&protocol_fee_owner, &debt_config.token.address, &recipient);

    assert_eq!(
        env.auths(),
        [(
            protocol_fee_owner.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "claim_protocol_fee"),
                    vec![
                        &env,
                        protocol_fee_owner.into_val(&env),
                        debt_config.token.address.into_val(&env),
                        recipient.into_val(&env)
                    ]
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #100)")]
fn should_fail_if_reserve_not_exists() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, true);
    let _ = generate_protocol_fee(&env, &sut, &debt_config.token.address, &borrower);
    let recipient = Address::generate(&env);
    let (token, _) = create_token_contract(&env, &sut.pool_admin);

    sut.pool
        .claim_protocol_fee(&sut.pool_admin, &token.address, &recipient);
}

#[test]
fn should_claim_fee() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, true);
    let expected_fee = generate_protocol_fee(&env, &sut, &debt_config.token.address, &borrower);
    let recipient = Address::generate(&env);

    let recipient_balance_before = debt_config.token.balance(&recipient);
    let recipient_stoken_balance_before = debt_config.s_token().balance(&recipient);
    let s_token_balance_before = debt_config.token.balance(&debt_config.s_token().address);
    let s_token_balance_internal_before = sut
        .pool
        .stoken_underlying_balance(&debt_config.s_token().address);
    let recipient_internal_balance_before =
        sut.pool.balance(&recipient, &debt_config.token.address);
    let fee_before = sut.pool.protocol_fee(&debt_config.token.address);

    sut.pool
        .claim_protocol_fee(&sut.pool_admin, &debt_config.token.address, &recipient);

    let recipient_balance_after = debt_config.token.balance(&recipient);
    let recipient_stoken_balance_after = debt_config.s_token().balance(&recipient);
    let s_token_balance_after = debt_config.token.balance(&debt_config.s_token().address);
    let s_token_balance_internal_after = sut
        .pool
        .stoken_underlying_balance(&debt_config.s_token().address);
    let recipient_internal_balance_after = sut.pool.balance(&recipient, &debt_config.token.address);
    let fee_after = sut.pool.protocol_fee(&debt_config.token.address);

    assert_eq!(
        recipient_balance_after - recipient_balance_before,
        expected_fee
    );
    assert_eq!(s_token_balance_before - s_token_balance_after, expected_fee);
    assert_eq!(
        recipient_stoken_balance_before,
        recipient_stoken_balance_after
    );

    assert_eq!(
        s_token_balance_internal_before,
        s_token_balance_internal_after
    );
    assert_eq!(
        recipient_internal_balance_before,
        recipient_internal_balance_after
    );

    assert_eq!(fee_before - fee_after, expected_fee);
    assert_eq!(fee_after, 0);
}

#[test]
fn should_claim_fee_rwa() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (liquidator, borrower) = fill_pool_six(&env, &sut);
    let recipient = Address::generate(&env);
    let collat_1_token = sut.reserves[0].token.address.clone();
    let rwa_token = sut.rwa_config().token.address.clone();
    let debt_token = sut.reserves[1].token.address.clone();

    sut.rwa_config()
        .token_admin
        .mint(&borrower, &100_000_000_000);

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
            liquidation_protocol_fee: 100,
        },
    );

    env.ledger().with_mut(|li| li.timestamp = 10_000);

    sut.pool
        .deposit(&borrower, &collat_1_token, &10_000_000_000);
    sut.pool.deposit(&borrower, &rwa_token, &100_000_000_000);
    sut.pool.borrow(&borrower, &debt_token, &800_000_000_000);

    sut.price_feed.init(
        &Asset::Stellar(debt_token),
        &vec![
            &env,
            PriceData {
                price: (18 * 10i128.pow(15)),
                timestamp: 0,
            },
        ],
    );

    sut.pool.liquidate(&liquidator, &borrower);

    let recipient_rwa_before = sut.rwa_config().token.balance(&recipient);
    let pool_rwa_before = sut.rwa_config().token.balance(&sut.pool.address);
    let fee_before = sut.pool.protocol_fee(&rwa_token);

    sut.pool
        .claim_protocol_fee(&sut.pool_admin, &rwa_token, &recipient);

    let recipient_rwa_after = sut.rwa_config().token.balance(&recipient);
    let pool_rwa_after = sut.rwa_config().token.balance(&sut.pool.address);
    let fee_after = sut.pool.protocol_fee(&rwa_token);

    assert_eq!(recipient_rwa_after - recipient_rwa_before, fee_before);
    assert_eq!(pool_rwa_before - pool_rwa_after, fee_before);
    assert_eq!(fee_after, 0);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_no_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, true);
    let _expected_fee = generate_protocol_fee(&env, &sut, &debt_config.token.address, &borrower);
    let recipient = Address::generate(&env);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::ClaimProtocolFee);
    let no_perm = Address::generate(&env);
    let permissioned = sut.pool.permissioned(&Permission::ClaimProtocolFee);

    assert!(permissioned.binary_search(&no_perm).is_err());

    sut.pool
        .claim_protocol_fee(&no_perm, &debt_config.token.address, &recipient);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_has_another_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, true);
    let _expected_fee = generate_protocol_fee(&env, &sut, &debt_config.token.address, &borrower);
    let recipient = Address::generate(&env);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::ClaimProtocolFee);
    let another_perm = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &another_perm,
        &Permission::CollateralReserveParams,
    );
    let permissioned = sut.pool.permissioned(&Permission::ClaimProtocolFee);

    assert!(permissioned.binary_search(&another_perm).is_err());

    sut.pool
        .claim_protocol_fee(&another_perm, &debt_config.token.address, &recipient);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_permission_revoked() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, true);
    let _expected_fee = generate_protocol_fee(&env, &sut, &debt_config.token.address, &borrower);
    let recipient = Address::generate(&env);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::ClaimProtocolFee);
    let revoked_perm = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &revoked_perm,
        &Permission::ClaimProtocolFee,
    );
    sut.pool.revoke_permission(
        &sut.pool_admin,
        &revoked_perm,
        &Permission::ClaimProtocolFee,
    );
    let permissioned = sut.pool.permissioned(&Permission::ClaimProtocolFee);

    assert!(permissioned.binary_search(&revoked_perm).is_err());

    sut.pool
        .claim_protocol_fee(&revoked_perm, &debt_config.token.address, &recipient);
}
