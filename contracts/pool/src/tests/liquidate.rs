use crate::tests::sut::{fill_pool, fill_pool_three, init_pool, DAY};
use crate::*;
use price_feed_interface::types::asset::Asset;
use price_feed_interface::types::price_data::PriceData;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, Events};
use soroban_sdk::{symbol_short, vec, IntoVal, Symbol};
use tests::sut::set_time;

use super::sut::fill_pool_six;

#[test]
fn should_require_authorized_caller() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, _) = fill_pool_three(&env, &sut);
    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        user_assets_limit: 4,
        min_collat_amount: 0,
        min_debt_amount: 0,
    });

    sut.pool.liquidate(&liquidator, &borrower, &false);

    assert_eq!(
        env.auths().pop().map(|f| f.1.function).unwrap(),
        AuthorizedFunction::Contract((
            sut.pool.address.clone(),
            symbol_short!("liquidate"),
            (liquidator.clone(), borrower.clone(), false).into_val(&env)
        )),
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn should_fail_when_pool_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, _) = fill_pool_three(&env, &sut);

    sut.pool.set_pause(&true);
    sut.pool.liquidate(&liquidator, &borrower, &false);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #101)")]
fn should_fail_when_reserve_deactivated() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, _) = fill_pool_three(&env, &sut);
    let collat_reserve = sut.reserves[0].token.address.clone();

    sut.pool.set_reserve_status(&collat_reserve, &false);
    sut.pool.liquidate(&liquidator, &borrower, &false);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #302)")]
fn should_fail_when_good_position() {
    let env = Env::default();
    env.mock_all_auths();

    let liquidator = Address::generate(&env);
    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    let position = sut.pool.account_position(&borrower);
    assert!(position.npv > 0, "test configuration");

    sut.pool.liquidate(&liquidator, &borrower, &false);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #204)")]
fn should_fail_when_have_debt_in_receiving_s_token() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);

    sut.pool
        .deposit(&liquidator, &debt_config.token.address, &500_000_000);
    sut.pool
        .borrow(&liquidator, &sut.reserves[0].token.address, &1_000_000);

    sut.pool.liquidate(&liquidator, &borrower, &true);
}

#[test]
#[should_panic(expected = "")]
fn should_fail_when_liquidator_has_not_enough_underlying_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);
    let token_address = debt_config.token.address.clone();

    sut.pool.deposit(&liquidator, &token_address, &999_990_000);
    sut.pool.liquidate(&liquidator, &borrower, &false);
}

#[test]
fn should_liquidate_reducing_position_to_healthy() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (liquidator, borrower) = fill_pool_six(&env, &sut);
    let collat_1_token = sut.reserves[0].token.address.clone();
    let collat_2_token = sut.reserves[2].token.address.clone();
    let debt_token = sut.reserves[1].token.address.clone();

    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        user_assets_limit: 4,
        min_collat_amount: 0,
        min_debt_amount: 0,
    });

    set_time(&env, &sut, 10_000, true);

    sut.pool
        .deposit(&borrower, &collat_1_token, &10_000_000_000);
    sut.pool
        .deposit(&borrower, &collat_2_token, &1_000_000_000_000);
    sut.pool.borrow(&borrower, &debt_token, &800_000_000_000);

    let borrower_token_0_before = sut.reserves[0].token.balance(&borrower);
    let borrower_token_1_before = sut.reserves[1].token.balance(&borrower);
    let borrower_token_2_before = sut.reserves[2].token.balance(&borrower);
    let borrower_stoken_0_before = sut.reserves[0].s_token().balance(&borrower);
    let borrower_stoken_1_before = sut.reserves[1].s_token().balance(&borrower);
    let borrower_stoken_2_before = sut.reserves[2].s_token().balance(&borrower);
    let borrower_dtoken_0_before = sut.reserves[0].debt_token().balance(&borrower);
    let borrower_dtoken_1_before = sut.reserves[1].debt_token().balance(&borrower);
    let borrower_dtoken_2_before = sut.reserves[2].debt_token().balance(&borrower);
    let borrower_account_position_before = sut.pool.account_position(&borrower);

    let liquidator_token_0_before = sut.reserves[0].token.balance(&liquidator);
    let liquidator_token_1_before = sut.reserves[1].token.balance(&liquidator);
    let liquidator_token_2_before = sut.reserves[2].token.balance(&liquidator);
    let liquidator_stoken_0_before = sut.reserves[0].s_token().balance(&liquidator);
    let liquidator_stoken_1_before = sut.reserves[1].s_token().balance(&liquidator);
    let liquidator_stoken_2_before = sut.reserves[2].s_token().balance(&liquidator);
    let liquidator_dtoken_0_before = sut.reserves[0].debt_token().balance(&liquidator);
    let liquidator_dtoken_1_before = sut.reserves[1].debt_token().balance(&liquidator);
    let liquidator_dtoken_2_before = sut.reserves[2].debt_token().balance(&liquidator);

    sut.price_feed.init(
        &Asset::Stellar(debt_token),
        &vec![
            &env,
            PriceData {
                price: (18 * 10i128.pow(15)),
                timestamp: 10_000,
            },
        ],
    );

    sut.pool.liquidate(&liquidator, &borrower, &false);

    let borrower_token_0_after = sut.reserves[0].token.balance(&borrower);
    let borrower_token_1_after = sut.reserves[1].token.balance(&borrower);
    let borrower_token_2_after = sut.reserves[2].token.balance(&borrower);
    let borrower_stoken_0_after = sut.reserves[0].s_token().balance(&borrower);
    let borrower_stoken_1_after = sut.reserves[1].s_token().balance(&borrower);
    let borrower_stoken_2_after = sut.reserves[2].s_token().balance(&borrower);
    let borrower_dtoken_0_after = sut.reserves[0].debt_token().balance(&borrower);
    let borrower_dtoken_1_after = sut.reserves[1].debt_token().balance(&borrower);
    let borrower_dtoken_2_after = sut.reserves[2].debt_token().balance(&borrower);
    let borrower_account_position_after = sut.pool.account_position(&borrower);

    let liquidator_token_0_after = sut.reserves[0].token.balance(&liquidator);
    let liquidator_token_1_after = sut.reserves[1].token.balance(&liquidator);
    let liquidator_token_2_after = sut.reserves[2].token.balance(&liquidator);
    let liquidator_stoken_0_after = sut.reserves[0].s_token().balance(&liquidator);
    let liquidator_stoken_1_after = sut.reserves[1].s_token().balance(&liquidator);
    let liquidator_stoken_2_after = sut.reserves[2].s_token().balance(&liquidator);
    let liquidator_dtoken_0_after = sut.reserves[0].debt_token().balance(&liquidator);
    let liquidator_dtoken_1_after = sut.reserves[1].debt_token().balance(&liquidator);
    let liquidator_dtoken_2_after = sut.reserves[2].debt_token().balance(&liquidator);

    assert_eq!(borrower_token_0_before, 0);
    assert_eq!(borrower_token_1_before, 1_800_000_000_000);
    assert_eq!(borrower_token_2_before, 0);
    assert_eq!(borrower_stoken_0_before, 10_000_000_000);
    assert_eq!(borrower_stoken_1_before, 0);
    assert_eq!(borrower_stoken_2_before, 1_000_000_000_000);
    assert_eq!(borrower_dtoken_0_before, 0);
    assert_eq!(borrower_dtoken_1_before, 800_000_000_000);
    assert_eq!(borrower_dtoken_2_before, 0);
    assert_eq!(borrower_account_position_before.npv, 3_999_493_504);
    assert_eq!(
        borrower_account_position_before.discounted_collateral,
        12_000_000_000
    );
    assert_eq!(borrower_account_position_before.debt, 8_000_506_496);

    assert_eq!(liquidator_token_0_before, 10_000_000_000);
    assert_eq!(liquidator_token_1_before, 1_000_000_000_000);
    assert_eq!(liquidator_token_2_before, 1_000_000_000_000);
    assert_eq!(liquidator_stoken_0_before, 0);
    assert_eq!(liquidator_stoken_1_before, 0);
    assert_eq!(liquidator_stoken_2_before, 0);
    assert_eq!(liquidator_dtoken_0_before, 0);
    assert_eq!(liquidator_dtoken_1_before, 0);
    assert_eq!(liquidator_dtoken_2_before, 0);

    assert_eq!(borrower_token_0_after, 0);
    assert_eq!(borrower_token_1_after, 1_800_000_000_000);
    assert_eq!(borrower_token_2_after, 0);
    assert_eq!(borrower_stoken_0_after, 0);
    assert_eq!(borrower_stoken_1_after, 0);
    assert_eq!(borrower_stoken_2_after, 456_547_338_939);
    assert_eq!(borrower_dtoken_0_after, 0);
    assert_eq!(borrower_dtoken_1_after, 114_129_609_050);
    assert_eq!(borrower_dtoken_2_after, 0);
    assert_eq!(borrower_account_position_after.npv, 684_821_007);
    assert_eq!(
        borrower_account_position_after.discounted_collateral,
        2_739_284_033
    );
    assert_eq!(borrower_account_position_after.debt, 2_054_463_026);

    assert_eq!(liquidator_token_0_after, 20_000_000_000);
    assert_eq!(liquidator_token_1_after, 314_086_185_223);
    assert_eq!(liquidator_token_2_after, 1_543_452_661_061);
    assert_eq!(liquidator_stoken_0_after, 0);
    assert_eq!(liquidator_stoken_1_after, 0);
    assert_eq!(liquidator_stoken_2_after, 0);
    assert_eq!(liquidator_dtoken_0_after, 0);
    assert_eq!(liquidator_dtoken_1_after, 0);
    assert_eq!(liquidator_dtoken_2_after, 0);
}

#[test]
fn should_liquidate_receiving_stokens_when_requested() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (liquidator, borrower) = fill_pool_six(&env, &sut);
    let collat_1_token = sut.reserves[0].token.address.clone();
    let collat_2_token = sut.reserves[2].token.address.clone();
    let debt_token = sut.reserves[1].token.address.clone();

    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        user_assets_limit: 4,
        min_collat_amount: 0,
        min_debt_amount: 0,
    });

    set_time(&env, &sut, 10_000, true);

    sut.pool
        .deposit(&borrower, &collat_1_token, &10_000_000_000);
    sut.pool
        .deposit(&borrower, &collat_2_token, &1_000_000_000_000);
    sut.pool.borrow(&borrower, &debt_token, &800_000_000_000);

    let borrower_token_0_before = sut.reserves[0].token.balance(&borrower);
    let borrower_token_1_before = sut.reserves[1].token.balance(&borrower);
    let borrower_token_2_before = sut.reserves[2].token.balance(&borrower);
    let borrower_stoken_0_before = sut.reserves[0].s_token().balance(&borrower);
    let borrower_stoken_1_before = sut.reserves[1].s_token().balance(&borrower);
    let borrower_stoken_2_before = sut.reserves[2].s_token().balance(&borrower);
    let borrower_dtoken_0_before = sut.reserves[0].debt_token().balance(&borrower);
    let borrower_dtoken_1_before = sut.reserves[1].debt_token().balance(&borrower);
    let borrower_dtoken_2_before = sut.reserves[2].debt_token().balance(&borrower);
    let borrower_account_position_before = sut.pool.account_position(&borrower);

    let liquidator_token_0_before = sut.reserves[0].token.balance(&liquidator);
    let liquidator_token_1_before = sut.reserves[1].token.balance(&liquidator);
    let liquidator_token_2_before = sut.reserves[2].token.balance(&liquidator);
    let liquidator_stoken_0_before = sut.reserves[0].s_token().balance(&liquidator);
    let liquidator_stoken_1_before = sut.reserves[1].s_token().balance(&liquidator);
    let liquidator_stoken_2_before = sut.reserves[2].s_token().balance(&liquidator);
    let liquidator_dtoken_0_before = sut.reserves[0].debt_token().balance(&liquidator);
    let liquidator_dtoken_1_before = sut.reserves[1].debt_token().balance(&liquidator);
    let liquidator_dtoken_2_before = sut.reserves[2].debt_token().balance(&liquidator);

    sut.price_feed.init(
        &Asset::Stellar(debt_token),
        &vec![
            &env,
            PriceData {
                price: (18 * 10i128.pow(15)),
                timestamp: 10_000,
            },
        ],
    );

    sut.pool.liquidate(&liquidator, &borrower, &true);

    let borrower_token_0_after = sut.reserves[0].token.balance(&borrower);
    let borrower_token_1_after = sut.reserves[1].token.balance(&borrower);
    let borrower_token_2_after = sut.reserves[2].token.balance(&borrower);
    let borrower_stoken_0_after = sut.reserves[0].s_token().balance(&borrower);
    let borrower_stoken_1_after = sut.reserves[1].s_token().balance(&borrower);
    let borrower_stoken_2_after = sut.reserves[2].s_token().balance(&borrower);
    let borrower_dtoken_0_after = sut.reserves[0].debt_token().balance(&borrower);
    let borrower_dtoken_1_after = sut.reserves[1].debt_token().balance(&borrower);
    let borrower_dtoken_2_after = sut.reserves[2].debt_token().balance(&borrower);
    let borrower_account_position_after = sut.pool.account_position(&borrower);

    let liquidator_token_0_after = sut.reserves[0].token.balance(&liquidator);
    let liquidator_token_1_after = sut.reserves[1].token.balance(&liquidator);
    let liquidator_token_2_after = sut.reserves[2].token.balance(&liquidator);
    let liquidator_stoken_0_after = sut.reserves[0].s_token().balance(&liquidator);
    let liquidator_stoken_1_after = sut.reserves[1].s_token().balance(&liquidator);
    let liquidator_stoken_2_after = sut.reserves[2].s_token().balance(&liquidator);
    let liquidator_dtoken_0_after = sut.reserves[0].debt_token().balance(&liquidator);
    let liquidator_dtoken_1_after = sut.reserves[1].debt_token().balance(&liquidator);
    let liquidator_dtoken_2_after = sut.reserves[2].debt_token().balance(&liquidator);

    assert_eq!(borrower_token_0_before, 0);
    assert_eq!(borrower_token_1_before, 1_800_000_000_000);
    assert_eq!(borrower_token_2_before, 0);
    assert_eq!(borrower_stoken_0_before, 10_000_000_000);
    assert_eq!(borrower_stoken_1_before, 0);
    assert_eq!(borrower_stoken_2_before, 1_000_000_000_000);
    assert_eq!(borrower_dtoken_0_before, 0);
    assert_eq!(borrower_dtoken_1_before, 800_000_000_000);
    assert_eq!(borrower_dtoken_2_before, 0);
    assert_eq!(borrower_account_position_before.npv, 3_999_493_504);
    assert_eq!(
        borrower_account_position_before.discounted_collateral,
        12_000_000_000
    );
    assert_eq!(borrower_account_position_before.debt, 8_000_506_496);

    assert_eq!(liquidator_token_0_before, 10_000_000_000);
    assert_eq!(liquidator_token_1_before, 1_000_000_000_000);
    assert_eq!(liquidator_token_2_before, 1_000_000_000_000);
    assert_eq!(liquidator_stoken_0_before, 0);
    assert_eq!(liquidator_stoken_1_before, 0);
    assert_eq!(liquidator_stoken_2_before, 0);
    assert_eq!(liquidator_dtoken_0_before, 0);
    assert_eq!(liquidator_dtoken_1_before, 0);
    assert_eq!(liquidator_dtoken_2_before, 0);

    assert_eq!(borrower_token_0_after, 0);
    assert_eq!(borrower_token_1_after, 1_800_000_000_000);
    assert_eq!(borrower_token_2_after, 0);
    assert_eq!(borrower_stoken_0_after, 0);
    assert_eq!(borrower_stoken_1_after, 0);
    assert_eq!(borrower_stoken_2_after, 456_547_338_939);
    assert_eq!(borrower_dtoken_0_after, 0);
    assert_eq!(borrower_dtoken_1_after, 114_129_609_050);
    assert_eq!(borrower_dtoken_2_after, 0);
    assert_eq!(borrower_account_position_after.npv, 684_821_007);
    assert_eq!(
        borrower_account_position_after.discounted_collateral,
        2_739_284_033
    );
    assert_eq!(borrower_account_position_after.debt, 2_054_463_026);

    assert_eq!(liquidator_token_0_after, 10_000_000_000);
    assert_eq!(liquidator_token_1_after, 314_086_185_223);
    assert_eq!(liquidator_token_2_after, 1_000_000_000_000);
    assert_eq!(liquidator_stoken_0_after, 10_000_000_000);
    assert_eq!(liquidator_stoken_1_after, 0);
    assert_eq!(liquidator_stoken_2_after, 543_452_661_061);
    assert_eq!(liquidator_dtoken_0_after, 0);
    assert_eq!(liquidator_dtoken_1_after, 0);
    assert_eq!(liquidator_dtoken_2_after, 0);
}

#[test]
fn should_fully_liquidate_when_gte_max_penalty() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (liquidator, borrower) = fill_pool_six(&env, &sut);
    let collat_1_token = sut.reserves[0].token.address.clone();
    let collat_2_token = sut.reserves[2].token.address.clone();
    let debt_token = sut.reserves[1].token.address.clone();

    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        user_assets_limit: 4,
        min_collat_amount: 0,
        min_debt_amount: 0,
    });

    set_time(&env, &sut, 10_000, true);

    sut.pool
        .deposit(&borrower, &collat_1_token, &10_000_000_000);
    sut.pool
        .deposit(&borrower, &collat_2_token, &1_000_000_000_000);
    sut.pool.borrow(&borrower, &debt_token, &800_000_000_000);

    let borrower_token_0_before = sut.reserves[0].token.balance(&borrower);
    let borrower_token_1_before = sut.reserves[1].token.balance(&borrower);
    let borrower_token_2_before = sut.reserves[2].token.balance(&borrower);
    let borrower_stoken_0_before = sut.reserves[0].s_token().balance(&borrower);
    let borrower_stoken_1_before = sut.reserves[1].s_token().balance(&borrower);
    let borrower_stoken_2_before = sut.reserves[2].s_token().balance(&borrower);
    let borrower_dtoken_0_before = sut.reserves[0].debt_token().balance(&borrower);
    let borrower_dtoken_1_before = sut.reserves[1].debt_token().balance(&borrower);
    let borrower_dtoken_2_before = sut.reserves[2].debt_token().balance(&borrower);
    let borrower_account_position_before = sut.pool.account_position(&borrower);

    let liquidator_token_0_before = sut.reserves[0].token.balance(&liquidator);
    let liquidator_token_1_before = sut.reserves[1].token.balance(&liquidator);
    let liquidator_token_2_before = sut.reserves[2].token.balance(&liquidator);
    let liquidator_stoken_0_before = sut.reserves[0].s_token().balance(&liquidator);
    let liquidator_stoken_1_before = sut.reserves[1].s_token().balance(&liquidator);
    let liquidator_stoken_2_before = sut.reserves[2].s_token().balance(&liquidator);
    let liquidator_dtoken_0_before = sut.reserves[0].debt_token().balance(&liquidator);
    let liquidator_dtoken_1_before = sut.reserves[1].debt_token().balance(&liquidator);
    let liquidator_dtoken_2_before = sut.reserves[2].debt_token().balance(&liquidator);

    sut.price_feed.init(
        &Asset::Stellar(debt_token),
        &vec![
            &env,
            PriceData {
                price: (2 * 10i128.pow(16)),
                timestamp: 10_000,
            },
        ],
    );

    sut.pool.liquidate(&liquidator, &borrower, &false);

    let borrower_token_0_after = sut.reserves[0].token.balance(&borrower);
    let borrower_token_1_after = sut.reserves[1].token.balance(&borrower);
    let borrower_token_2_after = sut.reserves[2].token.balance(&borrower);
    let borrower_stoken_0_after = sut.reserves[0].s_token().balance(&borrower);
    let borrower_stoken_1_after = sut.reserves[1].s_token().balance(&borrower);
    let borrower_stoken_2_after = sut.reserves[2].s_token().balance(&borrower);
    let borrower_dtoken_0_after = sut.reserves[0].debt_token().balance(&borrower);
    let borrower_dtoken_1_after = sut.reserves[1].debt_token().balance(&borrower);
    let borrower_dtoken_2_after = sut.reserves[2].debt_token().balance(&borrower);
    let borrower_account_position_after = sut.pool.account_position(&borrower);

    let liquidator_token_0_after = sut.reserves[0].token.balance(&liquidator);
    let liquidator_token_1_after = sut.reserves[1].token.balance(&liquidator);
    let liquidator_token_2_after = sut.reserves[2].token.balance(&liquidator);
    let liquidator_stoken_0_after = sut.reserves[0].s_token().balance(&liquidator);
    let liquidator_stoken_1_after = sut.reserves[1].s_token().balance(&liquidator);
    let liquidator_stoken_2_after = sut.reserves[2].s_token().balance(&liquidator);
    let liquidator_dtoken_0_after = sut.reserves[0].debt_token().balance(&liquidator);
    let liquidator_dtoken_1_after = sut.reserves[1].debt_token().balance(&liquidator);
    let liquidator_dtoken_2_after = sut.reserves[2].debt_token().balance(&liquidator);

    assert_eq!(borrower_token_0_before, 0);
    assert_eq!(borrower_token_1_before, 1_800_000_000_000);
    assert_eq!(borrower_token_2_before, 0);
    assert_eq!(borrower_stoken_0_before, 10_000_000_000);
    assert_eq!(borrower_stoken_1_before, 0);
    assert_eq!(borrower_stoken_2_before, 1_000_000_000_000);
    assert_eq!(borrower_dtoken_0_before, 0);
    assert_eq!(borrower_dtoken_1_before, 800_000_000_000);
    assert_eq!(borrower_dtoken_2_before, 0);
    assert_eq!(borrower_account_position_before.npv, 3_999_493_504);
    assert_eq!(
        borrower_account_position_before.discounted_collateral,
        12_000_000_000
    );
    assert_eq!(borrower_account_position_before.debt, 8_000_506_496);

    assert_eq!(liquidator_token_0_before, 10_000_000_000);
    assert_eq!(liquidator_token_1_before, 1_000_000_000_000);
    assert_eq!(liquidator_token_2_before, 1_000_000_000_000);
    assert_eq!(liquidator_stoken_0_before, 0);
    assert_eq!(liquidator_stoken_1_before, 0);
    assert_eq!(liquidator_stoken_2_before, 0);
    assert_eq!(liquidator_dtoken_0_before, 0);
    assert_eq!(liquidator_dtoken_1_before, 0);
    assert_eq!(liquidator_dtoken_2_before, 0);

    assert_eq!(borrower_token_0_after, 0);
    assert_eq!(borrower_token_1_after, 1_800_000_000_000);
    assert_eq!(borrower_token_2_after, 0);
    assert_eq!(borrower_stoken_0_after, 0);
    assert_eq!(borrower_stoken_1_after, 0);
    assert_eq!(borrower_stoken_2_after, 0);
    assert_eq!(borrower_dtoken_0_after, 0);
    assert_eq!(borrower_dtoken_1_after, 0);
    assert_eq!(borrower_dtoken_2_after, 0);
    assert_eq!(borrower_account_position_after.npv, 0);
    assert_eq!(borrower_account_position_after.discounted_collateral, 0);
    assert_eq!(borrower_account_position_after.debt, 0);

    assert_eq!(liquidator_token_0_after, 20_000_000_000);
    assert_eq!(liquidator_token_1_after, 199_949_350_400);
    assert_eq!(liquidator_token_2_after, 2_000_000_000_000);
    assert_eq!(liquidator_stoken_0_after, 0);
    assert_eq!(liquidator_stoken_1_after, 0);
    assert_eq!(liquidator_stoken_2_after, 0);
    assert_eq!(liquidator_dtoken_0_after, 0);
    assert_eq!(liquidator_dtoken_1_after, 0);
    assert_eq!(liquidator_dtoken_2_after, 0);
}

#[test]
fn should_fully_liquidate_receiving_stokens_when_requested_and_gte_penalty() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (liquidator, borrower) = fill_pool_six(&env, &sut);
    let collat_1_token = sut.reserves[0].token.address.clone();
    let collat_2_token = sut.reserves[2].token.address.clone();
    let debt_token = sut.reserves[1].token.address.clone();

    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        user_assets_limit: 4,
        min_collat_amount: 0,
        min_debt_amount: 0,
    });

    set_time(&env, &sut, 10_000, true);

    sut.pool
        .deposit(&borrower, &collat_1_token, &10_000_000_000);
    sut.pool
        .deposit(&borrower, &collat_2_token, &1_000_000_000_000);
    sut.pool.borrow(&borrower, &debt_token, &800_000_000_000);

    let borrower_token_0_before = sut.reserves[0].token.balance(&borrower);
    let borrower_token_1_before = sut.reserves[1].token.balance(&borrower);
    let borrower_token_2_before = sut.reserves[2].token.balance(&borrower);
    let borrower_stoken_0_before = sut.reserves[0].s_token().balance(&borrower);
    let borrower_stoken_1_before = sut.reserves[1].s_token().balance(&borrower);
    let borrower_stoken_2_before = sut.reserves[2].s_token().balance(&borrower);
    let borrower_dtoken_0_before = sut.reserves[0].debt_token().balance(&borrower);
    let borrower_dtoken_1_before = sut.reserves[1].debt_token().balance(&borrower);
    let borrower_dtoken_2_before = sut.reserves[2].debt_token().balance(&borrower);
    let borrower_account_position_before = sut.pool.account_position(&borrower);

    let liquidator_token_0_before = sut.reserves[0].token.balance(&liquidator);
    let liquidator_token_1_before = sut.reserves[1].token.balance(&liquidator);
    let liquidator_token_2_before = sut.reserves[2].token.balance(&liquidator);
    let liquidator_stoken_0_before = sut.reserves[0].s_token().balance(&liquidator);
    let liquidator_stoken_1_before = sut.reserves[1].s_token().balance(&liquidator);
    let liquidator_stoken_2_before = sut.reserves[2].s_token().balance(&liquidator);
    let liquidator_dtoken_0_before = sut.reserves[0].debt_token().balance(&liquidator);
    let liquidator_dtoken_1_before = sut.reserves[1].debt_token().balance(&liquidator);
    let liquidator_dtoken_2_before = sut.reserves[2].debt_token().balance(&liquidator);

    sut.price_feed.init(
        &Asset::Stellar(debt_token),
        &vec![
            &env,
            PriceData {
                price: (2 * 10i128.pow(16)),
                timestamp: 10_000,
            },
        ],
    );

    sut.pool.liquidate(&liquidator, &borrower, &true);

    let borrower_token_0_after = sut.reserves[0].token.balance(&borrower);
    let borrower_token_1_after = sut.reserves[1].token.balance(&borrower);
    let borrower_token_2_after = sut.reserves[2].token.balance(&borrower);
    let borrower_stoken_0_after = sut.reserves[0].s_token().balance(&borrower);
    let borrower_stoken_1_after = sut.reserves[1].s_token().balance(&borrower);
    let borrower_stoken_2_after = sut.reserves[2].s_token().balance(&borrower);
    let borrower_dtoken_0_after = sut.reserves[0].debt_token().balance(&borrower);
    let borrower_dtoken_1_after = sut.reserves[1].debt_token().balance(&borrower);
    let borrower_dtoken_2_after = sut.reserves[2].debt_token().balance(&borrower);
    let borrower_account_position_after = sut.pool.account_position(&borrower);

    let liquidator_token_0_after = sut.reserves[0].token.balance(&liquidator);
    let liquidator_token_1_after = sut.reserves[1].token.balance(&liquidator);
    let liquidator_token_2_after = sut.reserves[2].token.balance(&liquidator);
    let liquidator_stoken_0_after = sut.reserves[0].s_token().balance(&liquidator);
    let liquidator_stoken_1_after = sut.reserves[1].s_token().balance(&liquidator);
    let liquidator_stoken_2_after = sut.reserves[2].s_token().balance(&liquidator);
    let liquidator_dtoken_0_after = sut.reserves[0].debt_token().balance(&liquidator);
    let liquidator_dtoken_1_after = sut.reserves[1].debt_token().balance(&liquidator);
    let liquidator_dtoken_2_after = sut.reserves[2].debt_token().balance(&liquidator);

    assert_eq!(borrower_token_0_before, 0);
    assert_eq!(borrower_token_1_before, 1_800_000_000_000);
    assert_eq!(borrower_token_2_before, 0);
    assert_eq!(borrower_stoken_0_before, 10_000_000_000);
    assert_eq!(borrower_stoken_1_before, 0);
    assert_eq!(borrower_stoken_2_before, 1_000_000_000_000);
    assert_eq!(borrower_dtoken_0_before, 0);
    assert_eq!(borrower_dtoken_1_before, 800_000_000_000);
    assert_eq!(borrower_dtoken_2_before, 0);
    assert_eq!(borrower_account_position_before.npv, 3_999_493_504);
    assert_eq!(
        borrower_account_position_before.discounted_collateral,
        12_000_000_000
    );
    assert_eq!(borrower_account_position_before.debt, 8_000_506_496);

    assert_eq!(liquidator_token_0_before, 10_000_000_000);
    assert_eq!(liquidator_token_1_before, 1_000_000_000_000);
    assert_eq!(liquidator_token_2_before, 1_000_000_000_000);
    assert_eq!(liquidator_stoken_0_before, 0);
    assert_eq!(liquidator_stoken_1_before, 0);
    assert_eq!(liquidator_stoken_2_before, 0);
    assert_eq!(liquidator_dtoken_0_before, 0);
    assert_eq!(liquidator_dtoken_1_before, 0);
    assert_eq!(liquidator_dtoken_2_before, 0);

    assert_eq!(borrower_token_0_after, 0);
    assert_eq!(borrower_token_1_after, 1_800_000_000_000);
    assert_eq!(borrower_token_2_after, 0);
    assert_eq!(borrower_stoken_0_after, 0);
    assert_eq!(borrower_stoken_1_after, 0);
    assert_eq!(borrower_stoken_2_after, 0);
    assert_eq!(borrower_dtoken_0_after, 0);
    assert_eq!(borrower_dtoken_1_after, 0);
    assert_eq!(borrower_dtoken_2_after, 0);
    assert_eq!(borrower_account_position_after.npv, 0);
    assert_eq!(borrower_account_position_after.discounted_collateral, 0);
    assert_eq!(borrower_account_position_after.debt, 0);

    assert_eq!(liquidator_token_0_after, 10_000_000_000);
    assert_eq!(liquidator_token_1_after, 199_949_350_400);
    assert_eq!(liquidator_token_2_after, 1_000_000_000_000);
    assert_eq!(liquidator_stoken_0_after, 10_000_000_000);
    assert_eq!(liquidator_stoken_1_after, 0);
    assert_eq!(liquidator_stoken_2_after, 1_000_000_000_000);
    assert_eq!(liquidator_dtoken_0_after, 0);
    assert_eq!(liquidator_dtoken_1_after, 0);
    assert_eq!(liquidator_dtoken_2_after, 0);
}

#[test]
fn should_change_user_config() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (liquidator, borrower) = fill_pool_six(&env, &sut);
    let collat_1_token = sut.reserves[0].token.address.clone();
    let collat_2_token = sut.reserves[2].token.address.clone();
    let debt_token = sut.reserves[1].token.address.clone();

    let reserve_0 = sut
        .pool
        .get_reserve(&sut.reserves[0].token.address)
        .unwrap();
    let reserve_1 = sut
        .pool
        .get_reserve(&sut.reserves[1].token.address)
        .unwrap();
    let reserve_2 = sut
        .pool
        .get_reserve(&sut.reserves[2].token.address)
        .unwrap();

    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        user_assets_limit: 4,
        min_collat_amount: 0,
        min_debt_amount: 0,
    });

    set_time(&env, &sut, 10_000, false);

    sut.pool
        .deposit(&borrower, &collat_1_token, &10_000_000_000);
    sut.pool
        .deposit(&borrower, &collat_2_token, &1_000_000_000_000);
    sut.pool.borrow(&borrower, &debt_token, &800_000_000_000);

    sut.price_feed.init(
        &Asset::Stellar(debt_token),
        &vec![
            &env,
            PriceData {
                price: (18 * 10i128.pow(15)),
                timestamp: 10_000,
            },
        ],
    );

    let borrower_user_config = sut.pool.user_configuration(&borrower);

    let is_borrower_borrowed_token_0_before =
        borrower_user_config.is_borrowing(&env, reserve_0.get_id());
    let is_borrower_borrowed_token_1_before =
        borrower_user_config.is_borrowing(&env, reserve_1.get_id());
    let is_borrower_borrowed_token_2_before =
        borrower_user_config.is_borrowing(&env, reserve_2.get_id());
    let is_borrower_deposited_token_0_before =
        borrower_user_config.is_using_as_collateral(&env, reserve_0.get_id());
    let is_borrower_deposited_token_1_before =
        borrower_user_config.is_using_as_collateral(&env, reserve_1.get_id());
    let is_borrower_deposited_token_2_before =
        borrower_user_config.is_using_as_collateral(&env, reserve_2.get_id());
    let borrower_total_assets_before = borrower_user_config.total_assets();

    set_time(&env, &sut, 2 * DAY, false);

    sut.pool.liquidate(&liquidator, &borrower, &true);

    let liquidator_user_config = sut.pool.user_configuration(&liquidator);
    let borrower_user_config = sut.pool.user_configuration(&borrower);

    let is_liquidator_borrowed_token_0_after =
        liquidator_user_config.is_borrowing(&env, reserve_0.get_id());
    let is_liquidator_borrowed_token_1_after =
        liquidator_user_config.is_borrowing(&env, reserve_1.get_id());
    let is_liquidator_borrowed_token_2_after =
        liquidator_user_config.is_borrowing(&env, reserve_2.get_id());
    let is_liquidator_deposited_token_0_after =
        liquidator_user_config.is_using_as_collateral(&env, reserve_0.get_id());
    let is_liquidator_deposited_token_1_after =
        liquidator_user_config.is_using_as_collateral(&env, reserve_1.get_id());
    let is_liquidator_deposited_token_2_after =
        liquidator_user_config.is_using_as_collateral(&env, reserve_2.get_id());
    let liquidator_total_assets_after = liquidator_user_config.total_assets();

    let is_borrower_borrowed_token_0_after =
        borrower_user_config.is_borrowing(&env, reserve_0.get_id());
    let is_borrower_borrowed_token_1_after =
        borrower_user_config.is_borrowing(&env, reserve_1.get_id());
    let is_borrower_borrowed_token_2_after =
        borrower_user_config.is_borrowing(&env, reserve_2.get_id());
    let is_borrower_deposited_token_0_after =
        borrower_user_config.is_using_as_collateral(&env, reserve_0.get_id());
    let is_borrower_deposited_token_1_after =
        borrower_user_config.is_using_as_collateral(&env, reserve_1.get_id());
    let is_borrower_deposited_token_2_after =
        borrower_user_config.is_using_as_collateral(&env, reserve_2.get_id());
    let borrower_total_assets_after = borrower_user_config.total_assets();

    assert_eq!(is_borrower_borrowed_token_0_before, false);
    assert_eq!(is_borrower_borrowed_token_1_before, true);
    assert_eq!(is_borrower_borrowed_token_2_before, false);

    assert_eq!(is_borrower_deposited_token_0_before, true);
    assert_eq!(is_borrower_deposited_token_1_before, false);
    assert_eq!(is_borrower_deposited_token_2_before, true);
    assert_eq!(borrower_total_assets_before, 3);

    assert_eq!(is_borrower_borrowed_token_0_after, false);
    assert_eq!(is_borrower_borrowed_token_1_after, true);
    assert_eq!(is_borrower_borrowed_token_2_after, false);
    assert_eq!(borrower_total_assets_after, 2);

    assert_eq!(is_liquidator_borrowed_token_0_after, false);
    assert_eq!(is_liquidator_borrowed_token_1_after, false);
    assert_eq!(is_liquidator_borrowed_token_2_after, false);

    assert_eq!(is_borrower_deposited_token_0_after, false);
    assert_eq!(is_borrower_deposited_token_1_after, false);
    assert_eq!(is_borrower_deposited_token_2_after, true);
    assert_eq!(is_liquidator_deposited_token_0_after, true);
    assert_eq!(is_liquidator_deposited_token_1_after, false);
    assert_eq!(is_liquidator_deposited_token_2_after, true);
    assert_eq!(liquidator_total_assets_after, 2);
}

#[test]
fn should_affect_account_data() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (liquidator, borrower) = fill_pool_six(&env, &sut);
    let collat_1_token = sut.reserves[0].token.address.clone();
    let collat_2_token = sut.reserves[2].token.address.clone();
    let debt_token = sut.reserves[1].token.address.clone();

    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        user_assets_limit: 4,
        min_collat_amount: 0,
        min_debt_amount: 0,
    });

    set_time(&env, &sut, 10_000, true);

    sut.pool
        .deposit(&borrower, &collat_1_token, &10_000_000_000);
    sut.pool
        .deposit(&borrower, &collat_2_token, &1_000_000_000_000);
    sut.pool.borrow(&borrower, &debt_token, &800_000_000_000);

    sut.price_feed.init(
        &Asset::Stellar(debt_token),
        &vec![
            &env,
            PriceData {
                price: (18 * 10i128.pow(15)),
                timestamp: 10_000,
            },
        ],
    );

    let borrower_account_position_before = sut.pool.account_position(&borrower);

    set_time(&env, &sut, 2 * DAY + 1, false); // initial timestamp = grace period = 1

    sut.pool.liquidate(&liquidator, &borrower, &true);

    let liquidator_account_position_after = sut.pool.account_position(&liquidator);
    let borrower_account_position_after = sut.pool.account_position(&borrower);

    assert_eq!(
        borrower_account_position_before.discounted_collateral,
        12_000_000_000
    );
    assert_eq!(borrower_account_position_before.debt, 14_400_911_692);
    assert_eq!(borrower_account_position_before.npv, -2_400_911_692);

    assert_eq!(
        liquidator_account_position_after.discounted_collateral,
        9_319_109_724
    );
    assert_eq!(liquidator_account_position_after.debt, 0);
    assert_eq!(liquidator_account_position_after.npv, 9_319_109_724);

    assert_eq!(
        borrower_account_position_after.discounted_collateral,
        2_680_890_275
    );
    assert_eq!(borrower_account_position_after.debt, 2_008_842_827);
    assert_eq!(borrower_account_position_after.npv, 672_047_448);
}

#[test]
fn should_affect_coeffs() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (liquidator, borrower) = fill_pool_six(&env, &sut);
    let collat_1_token = sut.reserves[0].token.address.clone();
    let collat_2_token = sut.reserves[2].token.address.clone();
    let debt_token = sut.reserves[1].token.address.clone();

    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        user_assets_limit: 4,
        min_collat_amount: 0,
        min_debt_amount: 0,
    });

    set_time(&env, &sut, 10_000, false);

    sut.pool
        .deposit(&borrower, &collat_1_token, &10_000_000_000);
    sut.pool
        .deposit(&borrower, &collat_2_token, &1_000_000_000_000);
    sut.pool.borrow(&borrower, &debt_token, &800_000_000_000);

    sut.price_feed.init(
        &Asset::Stellar(debt_token),
        &vec![
            &env,
            PriceData {
                price: (18 * 10i128.pow(15)),
                timestamp: 10_000,
            },
        ],
    );

    set_time(&env, &sut, 4 * DAY, false);

    let asset_1 = sut.reserves[0].token.address.clone();
    let asset_2 = sut.reserves[1].token.address.clone();
    let asset_3 = sut.reserves[1].token.address.clone();

    let asset_1_collat_coeff_before = sut.pool.collat_coeff(&asset_1);
    let asset_1_debt_coeff_before = sut.pool.debt_coeff(&asset_1);
    let asset_2_collat_coeff_before = sut.pool.collat_coeff(&asset_2);
    let asset_2_debt_coeff_before = sut.pool.debt_coeff(&asset_2);
    let asset_3_collat_coeff_before = sut.pool.collat_coeff(&asset_3);
    let asset_3_debt_coeff_before = sut.pool.debt_coeff(&asset_3);

    set_time(&env, &sut, 5 * DAY, false);

    sut.pool.liquidate(&liquidator, &borrower, &false);

    set_time(&env, &sut, 6 * DAY, false);

    let asset_1_collat_coeff_after = sut.pool.collat_coeff(&asset_1);
    let asset_1_debt_coeff_after = sut.pool.debt_coeff(&asset_1);
    let asset_2_collat_coeff_after = sut.pool.collat_coeff(&asset_2);
    let asset_2_debt_coeff_after = sut.pool.debt_coeff(&asset_2);
    let asset_3_collat_coeff_after = sut.pool.collat_coeff(&asset_3);
    let asset_3_debt_coeff_after = sut.pool.debt_coeff(&asset_3);

    assert!(asset_1_collat_coeff_before == asset_1_collat_coeff_after);
    assert!(asset_1_debt_coeff_before == asset_1_debt_coeff_after);
    assert!(asset_2_collat_coeff_before < asset_2_collat_coeff_after);
    assert!(asset_2_debt_coeff_before > asset_2_debt_coeff_after);
    assert!(asset_3_collat_coeff_before < asset_3_collat_coeff_after);
    assert!(asset_3_debt_coeff_before > asset_3_debt_coeff_after);
}

#[test]
fn should_emit_events() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (liquidator, borrower) = fill_pool_six(&env, &sut);
    let collat_1_token = sut.reserves[0].token.address.clone();
    let collat_2_token = sut.reserves[2].token.address.clone();
    let debt_token = sut.reserves[1].token.address.clone();

    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        user_assets_limit: 4,
        min_collat_amount: 0,
        min_debt_amount: 0,
    });

    set_time(&env, &sut, 10_000, false);

    sut.pool
        .deposit(&borrower, &collat_1_token, &10_000_000_000);
    sut.pool
        .deposit(&borrower, &collat_2_token, &1_000_000_000_000);
    sut.pool.borrow(&borrower, &debt_token, &800_000_000_000);

    sut.price_feed.init(
        &Asset::Stellar(debt_token),
        &vec![
            &env,
            PriceData {
                price: (18 * 10i128.pow(15)),
                timestamp: 10_000,
            },
        ],
    );

    sut.pool.liquidate(&liquidator, &borrower, &false);

    let mut events = env.events().all();
    let event = events.pop_back_unchecked();

    assert_eq!(
        vec![&env, event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "liquidation"), borrower.clone()).into_val(&env),
                (12_346_441_522i128, 15_434_514_766i128).into_val(&env)
            ),
        ]
    );
}

#[test]
fn should_liquidate_rwa_collateral() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (liquidator, borrower) = fill_pool_six(&env, &sut);
    let collat_1_token = sut.reserves[0].token.address.clone();
    let rwa_token = sut.rwa_config().token.address.clone();
    let debt_token = sut.reserves[1].token.address.clone();

    sut.rwa_config()
        .token_admin
        .mint(&borrower, &100_000_000_000);

    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        user_assets_limit: 4,
        min_collat_amount: 0,
        min_debt_amount: 0,
    });

    set_time(&env, &sut, 10_000, false);

    sut.pool
        .deposit(&borrower, &collat_1_token, &10_000_000_000);
    sut.pool.deposit(&borrower, &rwa_token, &100_000_000_000);
    sut.pool.borrow(&borrower, &debt_token, &800_000_000_000);

    let borrower_rwa_before = sut.rwa_config().token.balance(&borrower);
    let liquidator_rwa_before = sut.rwa_config().token.balance(&liquidator);
    let pool_rwa_before = sut.rwa_config().token.balance(&sut.pool.address);

    sut.price_feed.init(
        &Asset::Stellar(debt_token),
        &vec![
            &env,
            PriceData {
                price: (18 * 10i128.pow(15)),
                timestamp: 10_000,
            },
        ],
    );

    sut.pool.liquidate(&liquidator, &borrower, &false);

    let borrower_rwa_after = sut.rwa_config().token.balance(&borrower);
    let liquidator_rwa_after = sut.rwa_config().token.balance(&liquidator);
    let pool_rwa_after = sut.rwa_config().token.balance(&sut.pool.address);

    assert_eq!(borrower_rwa_before, 0);
    assert_eq!(liquidator_rwa_before, 0);
    assert_eq!(pool_rwa_before, 100_000_000_000);

    assert_eq!(borrower_rwa_after, 0);
    assert!(liquidator_rwa_after > liquidator_rwa_before);
    assert!(pool_rwa_after < pool_rwa_before);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #205)")]
fn rwa_fail_when_exceed_assets_limit() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (liquidator, borrower) = fill_pool_six(&env, &sut);
    let collat_1_token = sut.reserves[0].token.address.clone();
    let collat_2_token = sut.reserves[2].token.address.clone();
    let debt_token = sut.reserves[1].token.address.clone();

    set_time(&env, &sut, 10_000, false);

    sut.pool
        .deposit(&borrower, &collat_1_token, &10_000_000_000);
    sut.pool
        .deposit(&borrower, &collat_2_token, &1_000_000_000_000);
    sut.pool.borrow(&borrower, &debt_token, &800_000_000_000);

    sut.price_feed.init(
        &Asset::Stellar(debt_token),
        &vec![
            &env,
            PriceData {
                price: (18 * 10i128.pow(15)),
                timestamp: 10_000,
            },
        ],
    );

    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        user_assets_limit: 1,
        min_collat_amount: 0,
        min_debt_amount: 0,
    });

    sut.pool.liquidate(&liquidator, &borrower, &true);
}

#[test]
fn should_not_panic_on_zero_collateral_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (liquidator, borrower) = fill_pool_six(&env, &sut);
    let high_priority_collat = &sut.reserves[0].token.address;
    let low_priority_collat = &sut.reserves[1].token.address;
    let debt_token = &sut.reserves[2].token.address;

    // deposit collat with high priority with price ~1 and amount 1e-9
    sut.pool.deposit(&borrower, high_priority_collat, &1);
    // deposit another collat
    sut.pool
        .deposit(&borrower, low_priority_collat, &1_000_000_000);
    sut.pool.borrow(&borrower, debt_token, &500_000_000);
    sut.price_feed.init(
        &Asset::Stellar(debt_token.clone()),
        &vec![
            &env,
            PriceData {
                price: 12_000_000_000_000_000,
                timestamp: 0,
            },
        ],
    );
    let _pos_before = sut.pool.account_position(&borrower);
    sut.pool.liquidate(&liquidator, &borrower, &true);
    let _pos_after = sut.pool.account_position(&borrower);

    assert!(_pos_before.npv < _pos_after.npv);
}

#[test]
fn should_round_debt_correctly() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        user_assets_limit: 4,
        min_collat_amount: 0,
        min_debt_amount: 0,
    });
    let (liquidator, borrower) = fill_pool_six(&env, &sut);
    let high_priority_collat = &sut.reserves[0].token.address;
    let low_priority_collat = &sut.reserves[1].token.address;
    let debt_token = &sut.reserves[2].token.address;

    // deposit collat with high priority with price ~1 and amount 1e-9
    sut.pool.deposit(&borrower, high_priority_collat, &1);
    // deposit another collat
    sut.pool
        .deposit(&borrower, low_priority_collat, &1_000_000_000);
    sut.pool.borrow(&borrower, debt_token, &400_000_000);
    sut.price_feed.init(
        &Asset::Stellar(debt_token.clone()),
        &vec![
            &env,
            PriceData {
                price: 16_000_000_000_000_000,
                timestamp: 0,
            },
        ],
    );
    let _pos_before = sut.pool.account_position(&borrower);
    sut.pool.liquidate(&liquidator, &borrower, &true);
    let _pos_after = sut.pool.account_position(&borrower);

    assert!(_pos_before.npv < _pos_after.npv);
}

#[test]
fn should_round_correctly_with_low_collateral() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        user_assets_limit: 4,
        min_collat_amount: 0,
        min_debt_amount: 0,
    });
    let (liquidator, borrower) = fill_pool_six(&env, &sut);
    let high_priority_collat = &sut.reserves[0].token.address;
    let low_priority_collat = &sut.reserves[1].token.address;
    let debt_token = &sut.reserves[2].token.address;

    // deposit collat with high priority with price ~1 and amount 1e-9
    sut.pool.deposit(&borrower, high_priority_collat, &1);
    // deposit another collat
    sut.pool
        .deposit(&borrower, low_priority_collat, &1_000_000_000);
    sut.pool.borrow(&borrower, debt_token, &400_000_000);
    sut.price_feed.init(
        &Asset::Stellar(debt_token.clone()),
        &vec![
            &env,
            PriceData {
                price: 20_000_000_000_000_000,
                timestamp: 0,
            },
        ],
    );
    let _pos_before = sut.pool.account_position(&borrower);
    sut.pool.liquidate(&liquidator, &borrower, &true);
    let _pos_after = sut.pool.account_position(&borrower);

    assert!(_pos_before.npv < _pos_after.npv);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #6)")]
fn should_fail_in_grace_period() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, _) = fill_pool_three(&env, &sut);

    sut.pool.set_pause(&true);
    sut.pool.set_pause(&false);
    sut.pool.liquidate(&liquidator, &borrower, &false);
}

#[test]
fn should_not_fail_after_grace_period() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let pause_info = sut.pool.pause_info();
    let start = env.ledger().timestamp();
    let gap = 500;
    let (_, borrower, liquidator, _) = fill_pool_three(&env, &sut);
    let borrower_pos_before = sut.pool.account_position(&borrower);

    sut.pool.set_pause(&true);
    set_time(&env, &sut, start + gap, false);
    sut.pool.set_pause(&false);
    set_time(
        &env,
        &sut,
        start + gap + pause_info.grace_period_secs,
        false,
    );

    sut.pool.liquidate(&liquidator, &borrower, &false);

    let borrower_npv_after = sut.pool.account_position(&borrower);

    assert!(borrower_npv_after.npv > borrower_pos_before.npv);
}
