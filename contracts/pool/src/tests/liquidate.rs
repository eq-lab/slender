use crate::tests::sut::{fill_pool, fill_pool_three, init_pool, DAY};
use crate::*;
use price_feed_interface::types::asset::Asset;
use price_feed_interface::types::price_data::PriceData;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, Events, Ledger};
use soroban_sdk::{symbol_short, vec, IntoVal, Symbol};

#[test]
fn should_require_authorized_caller() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);

    sut.pool
        .liquidate(&liquidator, &borrower, &debt_config.token.address, &false);

    assert_eq!(
        env.auths().pop().map(|f| f.1.function).unwrap(),
        AuthorizedFunction::Contract((
            sut.pool.address.clone(),
            symbol_short!("liquidate"),
            (
                liquidator.clone(),
                borrower.clone(),
                debt_config.token.address.clone(),
                false
            )
                .into_val(&env)
        )),
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn should_fail_when_pool_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);

    sut.pool.set_pause(&true);
    sut.pool
        .liquidate(&liquidator, &borrower, &debt_config.token.address, &false);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #101)")]
fn should_fail_when_reserve_deactivated() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);
    let collat_reserve = sut.reserves[0].token.address.clone();

    sut.pool.set_reserve_status(&collat_reserve, &false);
    sut.pool
        .liquidate(&liquidator, &borrower, &debt_config.token.address, &false);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #303)")]
fn should_fail_when_good_position() {
    let env = Env::default();
    env.mock_all_auths();

    let liquidator = Address::generate(&env);
    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);

    let position = sut.pool.account_position(&borrower);
    assert!(position.npv > 0, "test configuration");

    sut.pool
        .liquidate(&liquidator, &borrower, &debt_config.token.address, &false);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #106)")]
fn should_fail_when_oracle_price_is_negative() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);
    let token_address = debt_config.token.address.clone();

    sut.price_feed.init(
        &Asset::Stellar(token_address),
        &vec![
            &env,
            PriceData {
                price: -10_000_000_000,
                timestamp: 0,
            },
        ],
    );
    sut.pool
        .liquidate(&liquidator, &borrower, &debt_config.token.address, &false);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #308)")]
fn should_fail_when_not_enough_collateral() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);
    let token_address = debt_config.token.address.clone();

    sut.price_feed.init(
        &Asset::Stellar(token_address),
        &vec![
            &env,
            PriceData {
                price: (10i128.pow(16) * 2),
                timestamp: 0,
            },
        ],
    );
    sut.pool
        .liquidate(&liquidator, &borrower, &debt_config.token.address, &false);
}

#[test]
#[should_panic(expected = "")]
fn should_fail_when_liquidator_has_not_enough_underlying_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);
    let token_address = debt_config.token.address.clone();

    sut.pool.deposit(&liquidator, &token_address, &990_000_000);
    sut.pool
        .liquidate(&liquidator, &borrower, &debt_config.token.address, &false);
}

#[test]
fn should_liquidate_and_receive_collateral_partially() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);
    let token_address = debt_config.token.address.clone();

    env.ledger().with_mut(|li| li.timestamp = 4 * DAY);

    sut.reserves[2].token_admin.mint(&borrower, &100_000_000);
    sut.pool
        .deposit(&borrower, &sut.reserves[2].token.address, &50_000_000);
    sut.price_feed.init(
        &Asset::Stellar(token_address),
        &vec![
            &env,
            PriceData {
                price: (10i128.pow(16) * 2),
                timestamp: 0,
            },
        ],
    );

    env.ledger().with_mut(|li| li.timestamp = 5 * DAY);

    let underlying_0_supply_before = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[0].s_token.address);
    let underlying_2_supply_before = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[2].s_token.address);
    let borrower_stoken_0_balance_before = sut.reserves[0].s_token.balance(&borrower);
    let borrower_stoken_2_balance_before = sut.reserves[2].s_token.balance(&borrower);
    let borrower_debt_balance_before = sut.reserves[1].debt_token.balance(&borrower);
    let liquidator_repayment_balance_before = sut.reserves[1].token.balance(&liquidator);
    let liquidator_underlying_0_balance_before = sut.reserves[0].token.balance(&liquidator);
    let liquidator_underlying_2_balance_before = sut.reserves[2].token.balance(&liquidator);
    let liquidator_stoken_0_balance_before = sut.reserves[0].s_token.balance(&liquidator);
    let liquidator_stoken_2_balance_before = sut.reserves[2].s_token.balance(&liquidator);

    env.ledger().with_mut(|li| li.timestamp = 6 * DAY);

    sut.pool
        .liquidate(&liquidator, &borrower, &debt_config.token.address, &false);

    env.ledger().with_mut(|li| li.timestamp = 7 * DAY);

    let underlying_0_supply_after_partial_liquidation = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[0].s_token.address);
    let borrower_stoken_0_balance_after_partial_liquidation =
        sut.reserves[0].s_token.balance(&borrower);
    let borrower_debt_balance_after_partial_liquidation =
        sut.reserves[1].debt_token.balance(&borrower);
    let liquidator_repayment_balance_after_partial_liquidation =
        sut.reserves[1].token.balance(&liquidator);
    let liquidator_underlying_0_balance_after_partial_liquidation =
        sut.reserves[0].token.balance(&liquidator);
    let liquidator_stoken_0_balance_after_partial_liquidation =
        sut.reserves[0].s_token.balance(&liquidator);

    let _borrower_stoken_2_balance_after_full_liquidation =
        sut.reserves[2].s_token.balance(&borrower);
    let _underlying_2_supply_after_full_liquidation = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[2].s_token.address);

    sut.pool
        .liquidate(&liquidator, &borrower, &debt_config.token.address, &false);

    let underlying_2_supply_after_full_liquidation = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[2].s_token.address);
    let borrower_stoken_2_balance_after_full_liquidation =
        sut.reserves[2].s_token.balance(&borrower);
    let liquidator_underlying_2_balance_after_full_liquidation =
        sut.reserves[2].token.balance(&liquidator);
    let liquidator_stoken_2_balance_after_full_liquidation =
        sut.reserves[2].s_token.balance(&liquidator);

    assert_eq!(underlying_0_supply_before, 2_000_000);
    assert_eq!(underlying_2_supply_before, 150_000_000);
    assert_eq!(borrower_stoken_0_balance_before, 1_000_000);
    assert_eq!(borrower_stoken_2_balance_before, 50_000_000);
    assert_eq!(borrower_debt_balance_before, 60_000_001);
    assert_eq!(liquidator_repayment_balance_before, 1_000_000_000);
    assert_eq!(liquidator_underlying_0_balance_before, 0);
    assert_eq!(liquidator_underlying_2_balance_before, 0);
    assert_eq!(liquidator_stoken_0_balance_before, 0);
    assert_eq!(liquidator_stoken_2_balance_before, 0);

    assert_eq!(underlying_0_supply_after_partial_liquidation, 1_000_000);
    assert_eq!(borrower_stoken_0_balance_after_partial_liquidation, 0);
    assert_eq!(borrower_debt_balance_after_partial_liquidation, 15_055_059);
    assert_eq!(
        liquidator_repayment_balance_after_partial_liquidation,
        955_000_000
    );
    assert_eq!(
        liquidator_underlying_0_balance_after_partial_liquidation,
        1_000_000
    );
    assert_eq!(liquidator_stoken_0_balance_after_partial_liquidation, 0);

    assert_eq!(underlying_2_supply_after_full_liquidation, 116_854_000);
    assert_eq!(borrower_stoken_2_balance_after_full_liquidation, 16_854_000);
    assert_eq!(
        liquidator_underlying_2_balance_after_full_liquidation,
        33_146_000
    );
    assert_eq!(liquidator_stoken_2_balance_after_full_liquidation, 0);
}

#[test]
fn should_receive_stokens_when_requested() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);
    let token_address = debt_config.token.address.clone();

    env.ledger().with_mut(|li| li.timestamp = 4 * DAY);

    sut.reserves[2].token_admin.mint(&borrower, &100_000_000);
    sut.pool
        .deposit(&borrower, &sut.reserves[2].token.address, &50_000_000);

    sut.price_feed.init(
        &Asset::Stellar(token_address),
        &vec![
            &env,
            PriceData {
                price: (10i128.pow(16) * 2),
                timestamp: 0,
            },
        ],
    );

    env.ledger().with_mut(|li| li.timestamp = 5 * DAY);

    let underlying_1_supply_before = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[0].s_token.address);
    let underlying_2_supply_before = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[2].s_token.address);
    let borrower_stoken_1_balance_before = sut.reserves[0].s_token.balance(&borrower);
    let borrower_stoken_2_balance_before = sut.reserves[2].s_token.balance(&borrower);
    let borrower_debt_balance_before = sut.reserves[1].debt_token.balance(&borrower);
    let liquidator_repayment_balance_before = sut.reserves[1].token.balance(&liquidator);
    let liquidator_underlying_1_balance_before = sut.reserves[0].token.balance(&liquidator);
    let liquidator_underlying_2_balance_before = sut.reserves[2].token.balance(&liquidator);
    let liquidator_stoken_1_balance_before = sut.reserves[0].s_token.balance(&liquidator);
    let liquidator_stoken_2_balance_before = sut.reserves[2].s_token.balance(&liquidator);

    env.ledger().with_mut(|li| li.timestamp = 6 * DAY);

    sut.pool.liquidate(
        &liquidator,
        &borrower,
        &sut.reserves[1].token.address,
        &true,
    );

    env.ledger().with_mut(|li| li.timestamp = 7 * DAY);

    let underlying_1_supply_after_partial_liquidation = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[0].s_token.address);
    let underlying_2_supply_after_partial_liquidation = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[2].s_token.address);
    let borrower_stoken_1_balance_after_partial_liquidation =
        sut.reserves[0].s_token.balance(&borrower);
    let liquidator_underlying_1_balance_after_partial_liquidation =
        sut.reserves[0].token.balance(&liquidator);
    let liquidator_underlying_2_balance_after_partial_liquidation =
        sut.reserves[2].token.balance(&liquidator);
    let liquidator_stoken_1_balance_after_partial_liquidation =
        sut.reserves[0].s_token.balance(&liquidator);

    sut.pool.liquidate(
        &liquidator,
        &borrower,
        &sut.reserves[1].token.address,
        &true,
    );

    let borrower_stoken_2_balance_after_full_liquidation =
        sut.reserves[2].s_token.balance(&borrower);
    let borrower_debt_balance_after_full_liquidation =
        sut.reserves[1].debt_token.balance(&borrower);
    let liquidator_repayment_balance_after_full_liquidation =
        sut.reserves[1].token.balance(&liquidator);
    let liquidator_stoken_2_balance_after_full_liquidation =
        sut.reserves[2].s_token.balance(&liquidator);

    assert_eq!(underlying_1_supply_before, 2_000_000);
    assert_eq!(underlying_2_supply_before, 150_000_000);
    assert_eq!(borrower_stoken_1_balance_before, 1_000_000);
    assert_eq!(borrower_stoken_2_balance_before, 50_000_000);
    assert_eq!(borrower_debt_balance_before, 60_000_001);
    assert_eq!(liquidator_repayment_balance_before, 1_000_000_000);
    assert_eq!(liquidator_underlying_1_balance_before, 0);
    assert_eq!(liquidator_underlying_2_balance_before, 0);
    assert_eq!(liquidator_stoken_1_balance_before, 0);
    assert_eq!(liquidator_stoken_2_balance_before, 0);

    assert_eq!(underlying_1_supply_after_partial_liquidation, 2_000_000);
    assert_eq!(underlying_2_supply_after_partial_liquidation, 150_000_000);
    assert_eq!(borrower_stoken_1_balance_after_partial_liquidation, 0);
    assert_eq!(liquidator_underlying_1_balance_after_partial_liquidation, 0);
    assert_eq!(liquidator_underlying_2_balance_after_partial_liquidation, 0);
    assert_eq!(
        liquidator_stoken_1_balance_after_partial_liquidation,
        1_000_000
    );

    assert_eq!(borrower_stoken_2_balance_after_full_liquidation, 16_854_000);
    assert_eq!(borrower_debt_balance_after_full_liquidation, 0);
    assert_eq!(
        liquidator_repayment_balance_after_full_liquidation,
        939_933_588
    );
    assert_eq!(
        liquidator_stoken_2_balance_after_full_liquidation,
        33_146_000
    );
}

#[test]
fn should_change_user_config() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);

    env.ledger().with_mut(|li| li.timestamp = 4 * DAY);

    let token_address = debt_config.token.address.clone();
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

    sut.reserves[0].token_admin.mint(&liquidator, &300_000_000);
    sut.reserves[2].token_admin.mint(&borrower, &300_000_000);

    sut.pool
        .deposit(&liquidator, &debt_config.token.address, &100_000_000);
    sut.pool
        .borrow(&liquidator, &sut.reserves[2].token.address, &20_000_000);
    sut.pool
        .deposit(&borrower, &sut.reserves[2].token.address, &120_000_000);
    sut.price_feed.init(
        &Asset::Stellar(token_address),
        &vec![
            &env,
            PriceData {
                price: (10i128.pow(16) * 3_200_000_000 / 1_000_000_000),
                timestamp: 0,
            },
        ],
    );

    env.ledger().with_mut(|li| li.timestamp = 5 * DAY);

    let liquidator_user_config = sut.pool.user_configuration(&liquidator);
    let borrower_user_config = sut.pool.user_configuration(&borrower);

    let is_liquidator_borrowed_asset_2_before =
        liquidator_user_config.is_borrowing(&env, reserve_2.get_id());
    let is_liquidator_deposited_asset_2_before =
        liquidator_user_config.is_using_as_collateral(&env, reserve_2.get_id());
    let is_borrower_borrowed_asset_1_before =
        borrower_user_config.is_borrowing(&env, reserve_1.get_id());
    let is_borrower_deposited_asset_0_before =
        borrower_user_config.is_using_as_collateral(&env, reserve_0.get_id());
    let is_borrower_deposited_asset_2_before =
        borrower_user_config.is_using_as_collateral(&env, reserve_2.get_id());

    env.ledger().with_mut(|li| li.timestamp = 6 * DAY);

    sut.pool
        .liquidate(&liquidator, &borrower, &debt_config.token.address, &true);

    let liquidator_user_config_after_partial_liquidation = sut.pool.user_configuration(&liquidator);
    let borrower_user_config_after_partial_liquidation = sut.pool.user_configuration(&borrower);

    let is_liquidator_borrowed_asset_2_after_partial_liquidation =
        liquidator_user_config_after_partial_liquidation.is_borrowing(&env, reserve_2.get_id());
    let is_liquidator_deposited_asset_2_after_partial_liquidation =
        liquidator_user_config_after_partial_liquidation
            .is_using_as_collateral(&env, reserve_2.get_id());
    let is_borrower_borrowed_asset_1_after_partial_liquidation =
        borrower_user_config_after_partial_liquidation.is_borrowing(&env, reserve_1.get_id());
    let is_borrower_deposited_asset_0_after_partial_liquidation =
        borrower_user_config_after_partial_liquidation
            .is_using_as_collateral(&env, reserve_0.get_id());
    let is_borrower_deposited_asset_2_after_partial_liquidation =
        borrower_user_config_after_partial_liquidation
            .is_using_as_collateral(&env, reserve_2.get_id());

    sut.pool
        .liquidate(&liquidator, &borrower, &debt_config.token.address, &false);

    let liquidator_user_config_after_full_liquidation = sut.pool.user_configuration(&liquidator);
    let borrower_user_config_after_full_liquidation = sut.pool.user_configuration(&borrower);

    let is_liquidator_borrowed_asset_2_after_full_liquidation =
        liquidator_user_config_after_full_liquidation.is_borrowing(&env, reserve_2.get_id());
    let is_liquidator_deposited_asset_2_after_full_liquidation =
        liquidator_user_config_after_full_liquidation
            .is_using_as_collateral(&env, reserve_2.get_id());
    let is_borrower_borrowed_asset_1_after_full_liquidation =
        borrower_user_config_after_full_liquidation.is_borrowing(&env, reserve_1.get_id());
    let is_borrower_deposited_asset_0_after_full_liquidation =
        borrower_user_config_after_full_liquidation
            .is_using_as_collateral(&env, reserve_0.get_id());
    let is_borrower_deposited_asset_2_after_full_liquidation =
        borrower_user_config_after_full_liquidation
            .is_using_as_collateral(&env, reserve_2.get_id());

    assert_eq!(is_liquidator_borrowed_asset_2_before, true);
    assert_eq!(is_liquidator_deposited_asset_2_before, false);
    assert_eq!(is_borrower_borrowed_asset_1_before, true);
    assert_eq!(is_borrower_deposited_asset_0_before, true);
    assert_eq!(is_borrower_deposited_asset_2_before, true);

    assert_eq!(
        is_liquidator_borrowed_asset_2_after_partial_liquidation,
        true
    );
    assert_eq!(
        is_liquidator_deposited_asset_2_after_partial_liquidation,
        false
    );
    assert_eq!(is_borrower_borrowed_asset_1_after_partial_liquidation, true);
    assert_eq!(
        is_borrower_deposited_asset_0_after_partial_liquidation,
        false
    );
    assert_eq!(
        is_borrower_deposited_asset_2_after_partial_liquidation,
        true
    );

    assert_eq!(is_liquidator_borrowed_asset_2_after_full_liquidation, true);
    assert_eq!(
        is_liquidator_deposited_asset_2_after_full_liquidation,
        false
    );
    assert_eq!(is_borrower_borrowed_asset_1_after_full_liquidation, false);
    assert_eq!(is_borrower_deposited_asset_0_after_full_liquidation, false);
    assert_eq!(is_borrower_deposited_asset_2_after_full_liquidation, true);
}

#[test]
fn should_affect_account_data() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);

    env.ledger().with_mut(|li| li.timestamp = 4 * DAY);

    let borrower_account_position_before = sut.pool.account_position(&borrower);

    sut.pool
        .liquidate(&liquidator, &borrower, &debt_config.token.address, &true);

    env.ledger().with_mut(|li| li.timestamp = 5 * DAY);

    let liquidator_account_position_after = sut.pool.account_position(&liquidator);
    let borrower_account_position_after = sut.pool.account_position(&borrower);

    assert!(
        borrower_account_position_before.discounted_collateral
            > borrower_account_position_after.discounted_collateral
    );
    assert!(borrower_account_position_before.debt > borrower_account_position_after.debt);
    assert!(borrower_account_position_before.npv < borrower_account_position_after.npv);

    assert!(liquidator_account_position_after.discounted_collateral > 0);
    assert!(liquidator_account_position_after.debt == 0);
    assert!(liquidator_account_position_after.npv > 0);
}

#[test]
fn should_affect_coeffs() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);

    env.ledger().with_mut(|li| li.timestamp = 4 * DAY);

    let asset_1 = sut.reserves[0].token.address.clone();
    let asset_2 = sut.reserves[1].token.address.clone();

    let asset_1_collat_coeff_before = sut.pool.collat_coeff(&asset_1);
    let asset_1_debt_coeff_before = sut.pool.debt_coeff(&asset_1);
    let asset_2_collat_coeff_before = sut.pool.collat_coeff(&asset_2);
    let asset_2_debt_coeff_before = sut.pool.debt_coeff(&asset_2);

    env.ledger().with_mut(|li| li.timestamp = 5 * DAY);

    sut.pool
        .liquidate(&liquidator, &borrower, &debt_config.token.address, &false);

    env.ledger().with_mut(|li| li.timestamp = 6 * DAY);

    let asset_1_collat_coeff_after = sut.pool.collat_coeff(&asset_1);
    let asset_1_debt_coeff_after = sut.pool.debt_coeff(&asset_1);
    let asset_2_collat_coeff_after = sut.pool.collat_coeff(&asset_2);
    let asset_2_debt_coeff_after = sut.pool.debt_coeff(&asset_2);

    assert!(asset_1_collat_coeff_before == asset_1_collat_coeff_after);
    assert!(asset_1_debt_coeff_before == asset_1_debt_coeff_after);
    assert!(asset_2_collat_coeff_before < asset_2_collat_coeff_after);
    assert!(asset_2_debt_coeff_before > asset_2_debt_coeff_after);
}

#[test]
fn should_emit_events() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);

    env.ledger().with_mut(|li| li.timestamp = 4 * DAY);

    sut.pool
        .liquidate(&liquidator, &borrower, &debt_config.token.address, &false);

    let mut events = env.events().all();
    let event = events.pop_back_unchecked();

    assert_eq!(
        vec![&env, event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "liquidation"), borrower.clone()).into_val(&env),
                (600_489i128, 660_537i128).into_val(&env)
            ),
        ]
    );
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

    sut.pool
        .liquidate(&liquidator, &borrower, &debt_config.token.address, &true);
}
