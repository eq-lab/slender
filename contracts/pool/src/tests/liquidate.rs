use crate::tests::sut::{fill_pool, fill_pool_three, init_pool, DAY};
use crate::*;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, Events, Ledger};
use soroban_sdk::{symbol_short, vec, IntoVal, Symbol};

#[test]
fn should_require_authorized_caller() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, _) = fill_pool_three(&env, &sut);

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

    // assert_eq!(
    //     sut.pool
    //         .try_liquidate(&liquidator, &borrower, &false)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::Paused
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_reserve_deactivated() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, _) = fill_pool_three(&env, &sut);
    let collat_reserve = sut.reserves[0].token.address.clone();

    sut.pool.set_reserve_status(&collat_reserve, &false);
    sut.pool.liquidate(&liquidator, &borrower, &false);

    // assert_eq!(
    //     sut.pool
    //         .try_liquidate(&liquidator, &borrower, &false)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::NoActiveReserve
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_good_position() {
    let env = Env::default();
    env.mock_all_auths();

    let liquidator = Address::random(&env);
    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    let position = sut.pool.account_position(&borrower);
    assert!(position.npv > 0, "test configuration");

    sut.pool.liquidate(&liquidator, &borrower, &false);

    // assert_eq!(
    //     sut.pool
    //         .try_liquidate(&liquidator, &borrower, &false)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::GoodPosition
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_oracle_price_is_negative() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);
    let token_address = debt_config.token.address.clone();

    sut.price_feed.set_price(&token_address, &-1_000);
    sut.pool.liquidate(&liquidator, &borrower, &false);

    // assert_eq!(
    //     sut.pool
    //         .try_liquidate(&liquidator, &borrower, &false)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::InvalidAssetPrice
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn sould_fail_when_not_enough_collateral() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);
    let token_address = debt_config.token.address.clone();

    sut.price_feed
        .set_price(&token_address, &(10i128.pow(sut.price_feed.decimals()) * 2));
    sut.pool.liquidate(&liquidator, &borrower, &false);

    // assert_eq!(
    //     sut.pool
    //         .try_liquidate(&liquidator, &borrower, &false)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::NotEnoughCollateral
    // );
}

#[test]
#[should_panic(expected = "")]
fn sould_fail_when_liquidator_has_not_enough_underlying_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);
    let token_address = debt_config.token.address.clone();

    sut.pool.deposit(&liquidator, &token_address, &990_000_000);
    sut.pool.liquidate(&liquidator, &borrower, &false);

    // assert_eq!(
    //     sut.pool
    //         .try_liquidate(&liquidator, &borrower, &false)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::NotEnoughCollateral
    // );
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
        .deposit(&borrower, &sut.reserves[2].token.address, &90_000_000);
    sut.price_feed
        .set_price(&token_address, &(10i128.pow(sut.price_feed.decimals()) * 2));

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

    sut.pool.liquidate(&liquidator, &borrower, &false);

    env.ledger().with_mut(|li| li.timestamp = 7 * DAY);

    let underlying_1_supply_after = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[0].s_token.address);
    let underlying_2_supply_after = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[2].s_token.address);
    let borrower_stoken_1_balance_after = sut.reserves[0].s_token.balance(&borrower);
    let borrower_stoken_2_balance_after = sut.reserves[2].s_token.balance(&borrower);
    let borrower_debt_balance_after = sut.reserves[1].debt_token.balance(&borrower);
    let liquidator_repayment_balance_after = sut.reserves[1].token.balance(&liquidator);
    let liquidator_underlying_1_balance_after = sut.reserves[0].token.balance(&liquidator);
    let liquidator_underlying_2_balance_after = sut.reserves[2].token.balance(&liquidator);
    let liquidator_stoken_1_balance_after = sut.reserves[0].s_token.balance(&liquidator);
    let liquidator_stoken_2_balance_after = sut.reserves[2].s_token.balance(&liquidator);

    assert_eq!(underlying_1_supply_before, 200_000_000);
    assert_eq!(underlying_2_supply_before, 190_000_000);
    assert_eq!(borrower_stoken_1_balance_before, 100_000_000);
    assert_eq!(borrower_stoken_2_balance_before, 90_000_000);
    assert_eq!(borrower_debt_balance_before, 60_000_000);
    assert_eq!(liquidator_repayment_balance_before, 1_000_000_000);
    assert_eq!(liquidator_underlying_1_balance_before, 0);
    assert_eq!(liquidator_underlying_2_balance_before, 0);
    assert_eq!(liquidator_stoken_1_balance_before, 0);
    assert_eq!(liquidator_stoken_2_balance_before, 0);

    assert_eq!(underlying_1_supply_after, 100_000_000);
    assert_eq!(underlying_2_supply_after, 157_838_303);
    assert_eq!(borrower_stoken_1_balance_after, 0);
    assert_eq!(borrower_stoken_2_balance_after, 57_838_303);
    assert_eq!(borrower_debt_balance_after, 0);
    assert_eq!(liquidator_repayment_balance_after, 939_926_501);
    assert_eq!(liquidator_underlying_1_balance_after, 100_000_000);
    assert_eq!(liquidator_underlying_2_balance_after, 32_161_697);
    assert_eq!(liquidator_stoken_1_balance_after, 0);
    assert_eq!(liquidator_stoken_2_balance_after, 0);
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
        .deposit(&borrower, &sut.reserves[2].token.address, &90_000_000);
    sut.price_feed
        .set_price(&token_address, &(10i128.pow(sut.price_feed.decimals()) * 2));

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

    sut.pool.liquidate(&liquidator, &borrower, &true);

    env.ledger().with_mut(|li| li.timestamp = 7 * DAY);

    let underlying_1_supply_after = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[0].s_token.address);
    let underlying_2_supply_after = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[2].s_token.address);
    let borrower_stoken_1_balance_after = sut.reserves[0].s_token.balance(&borrower);
    let borrower_stoken_2_balance_after = sut.reserves[2].s_token.balance(&borrower);
    let borrower_debt_balance_after = sut.reserves[1].debt_token.balance(&borrower);
    let liquidator_repayment_balance_after = sut.reserves[1].token.balance(&liquidator);
    let liquidator_underlying_1_balance_after = sut.reserves[0].token.balance(&liquidator);
    let liquidator_underlying_2_balance_after = sut.reserves[2].token.balance(&liquidator);
    let liquidator_stoken_1_balance_after = sut.reserves[0].s_token.balance(&liquidator);
    let liquidator_stoken_2_balance_after = sut.reserves[2].s_token.balance(&liquidator);

    assert_eq!(underlying_1_supply_before, 200_000_000);
    assert_eq!(underlying_2_supply_before, 190_000_000);
    assert_eq!(borrower_stoken_1_balance_before, 100_000_000);
    assert_eq!(borrower_stoken_2_balance_before, 90_000_000);
    assert_eq!(borrower_debt_balance_before, 60_000_000);
    assert_eq!(liquidator_repayment_balance_before, 1_000_000_000);
    assert_eq!(liquidator_underlying_1_balance_before, 0);
    assert_eq!(liquidator_underlying_2_balance_before, 0);
    assert_eq!(liquidator_stoken_1_balance_before, 0);
    assert_eq!(liquidator_stoken_2_balance_before, 0);

    assert_eq!(underlying_1_supply_after, 200_000_000);
    assert_eq!(underlying_2_supply_after, 190_000_000);
    assert_eq!(borrower_stoken_1_balance_after, 0);
    assert_eq!(borrower_stoken_2_balance_after, 57_838_303);
    assert_eq!(borrower_debt_balance_after, 0);
    assert_eq!(liquidator_repayment_balance_after, 939_926_501);
    assert_eq!(liquidator_underlying_1_balance_after, 0);
    assert_eq!(liquidator_underlying_2_balance_after, 0);
    assert_eq!(liquidator_stoken_1_balance_after, 100_000_000);
    assert_eq!(liquidator_stoken_2_balance_after, 32_161_697);
}

#[test]
fn should_repay_liquidator_debt_when_stokens_requested() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, debt_config) = fill_pool_three(&env, &sut);
    let token_address = debt_config.token.address.clone();
    let treasury = sut.pool.treasury();

    env.ledger().with_mut(|li| li.timestamp = 4 * DAY);

    sut.reserves[0].token_admin.mint(&liquidator, &100_000_000);
    sut.reserves[2].token_admin.mint(&borrower, &100_000_000);

    sut.pool
        .deposit(&liquidator, &debt_config.token.address, &100_000_000);
    sut.pool
        .borrow(&liquidator, &sut.reserves[0].token.address, &20_000_000);
    sut.pool
        .deposit(&borrower, &sut.reserves[2].token.address, &90_000_000);
    sut.price_feed
        .set_price(&token_address, &(10i128.pow(sut.price_feed.decimals()) * 2));

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
    let liquidator_debt_balance_before = sut.reserves[0].debt_token.balance(&liquidator);
    let liquidator_repayment_balance_before = sut.reserves[1].token.balance(&liquidator);
    let liquidator_underlying_1_balance_before = sut.reserves[0].token.balance(&liquidator);
    let liquidator_underlying_2_balance_before = sut.reserves[2].token.balance(&liquidator);
    let liquidator_stoken_1_balance_before = sut.reserves[0].s_token.balance(&liquidator);
    let liquidator_stoken_2_balance_before = sut.reserves[2].s_token.balance(&liquidator);
    let treasury_underlying_balance_before = sut.reserves[0].token.balance(&treasury);
    let collat_token_0_total_supply_before = sut.reserves[0].s_token.total_supply();
    let pool_collat_token_0_total_supply_before = sut
        .pool
        .token_total_supply(&sut.reserves[0].s_token.address);
    let debt_token_0_total_supply_before = sut.reserves[0].debt_token.total_supply();
    let pool_debt_token_0_total_supply_before = sut
        .pool
        .token_total_supply(&sut.reserves[0].debt_token.address);
    let collat_token_2_total_supply_before = sut.reserves[2].s_token.total_supply();
    let pool_collat_token_2_total_supply_before = sut
        .pool
        .token_total_supply(&sut.reserves[2].s_token.address);
    let debt_token_2_total_supply_before = sut.reserves[2].debt_token.total_supply();
    let pool_debt_token_2_total_supply_before = sut
        .pool
        .token_total_supply(&sut.reserves[2].debt_token.address);

    env.ledger().with_mut(|li| li.timestamp = 6 * DAY);

    sut.pool.liquidate(&liquidator, &borrower, &true);

    env.ledger().with_mut(|li| li.timestamp = 7 * DAY);

    let underlying_1_supply_after = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[0].s_token.address);
    let underlying_2_supply_after = sut
        .pool
        .stoken_underlying_balance(&sut.reserves[2].s_token.address);
    let borrower_stoken_1_balance_after = sut.reserves[0].s_token.balance(&borrower);
    let borrower_stoken_2_balance_after = sut.reserves[2].s_token.balance(&borrower);
    let borrower_debt_balance_after = sut.reserves[1].debt_token.balance(&borrower);
    let liquidator_debt_balance_after = sut.reserves[0].debt_token.balance(&liquidator);
    let liquidator_repayment_balance_after = sut.reserves[1].token.balance(&liquidator);
    let liquidator_underlying_1_balance_after = sut.reserves[0].token.balance(&liquidator);
    let liquidator_underlying_2_balance_after = sut.reserves[2].token.balance(&liquidator);
    let liquidator_stoken_1_balance_after = sut.reserves[0].s_token.balance(&liquidator);
    let liquidator_stoken_2_balance_after = sut.reserves[2].s_token.balance(&liquidator);
    let treasury_underlying_balance_after = sut.reserves[0].token.balance(&treasury);
    let collat_token_0_total_supply_after = sut.reserves[0].s_token.total_supply();
    let pool_collat_token_0_total_supply_after = sut
        .pool
        .token_total_supply(&sut.reserves[0].s_token.address);
    let debt_token_0_total_supply_after = sut.reserves[0].debt_token.total_supply();
    let pool_debt_token_0_total_supply_after = sut
        .pool
        .token_total_supply(&sut.reserves[0].debt_token.address);
    let collat_token_2_total_supply_after = sut.reserves[2].s_token.total_supply();
    let pool_collat_token_2_total_supply_after = sut
        .pool
        .token_total_supply(&sut.reserves[2].s_token.address);
    let debt_token_2_total_supply_after = sut.reserves[2].debt_token.total_supply();
    let pool_debt_token_2_total_supply_after = sut
        .pool
        .token_total_supply(&sut.reserves[2].debt_token.address);

    assert_eq!(underlying_1_supply_before, 180_000_000);
    assert_eq!(underlying_2_supply_before, 190_000_000);
    assert_eq!(borrower_stoken_1_balance_before, 100_000_000);
    assert_eq!(borrower_stoken_2_balance_before, 90_000_000);
    assert_eq!(borrower_debt_balance_before, 60_000_000);
    assert_eq!(liquidator_debt_balance_before, 20_000_000);
    assert_eq!(liquidator_repayment_balance_before, 900_000_000);
    assert_eq!(liquidator_underlying_1_balance_before, 120_000_000);
    assert_eq!(liquidator_underlying_2_balance_before, 0);
    assert_eq!(liquidator_stoken_1_balance_before, 0);
    assert_eq!(liquidator_stoken_2_balance_before, 0);
    assert_eq!(treasury_underlying_balance_before, 0);
    assert_eq!(
        collat_token_0_total_supply_before,
        pool_collat_token_0_total_supply_before
    );
    assert_eq!(
        debt_token_0_total_supply_before,
        pool_debt_token_0_total_supply_before
    );
    assert_eq!(
        collat_token_2_total_supply_before,
        pool_collat_token_2_total_supply_before
    );
    assert_eq!(
        debt_token_2_total_supply_before,
        pool_debt_token_2_total_supply_before
    );

    assert_eq!(underlying_1_supply_after, 179_994_206);
    assert_eq!(underlying_2_supply_after, 190_000_000);
    assert_eq!(borrower_stoken_1_balance_after, 0);
    assert_eq!(borrower_stoken_2_balance_after, 57_900_798);
    assert_eq!(borrower_debt_balance_after, 0);
    assert_eq!(liquidator_debt_balance_after, 0);
    assert_eq!(liquidator_repayment_balance_after, 839_953_606);
    assert_eq!(liquidator_underlying_1_balance_after, 120_000_000);
    assert_eq!(liquidator_underlying_2_balance_after, 0);
    assert_eq!(liquidator_stoken_1_balance_after, 79_994_208);
    assert_eq!(liquidator_stoken_2_balance_after, 32_099_202);
    assert_eq!(treasury_underlying_balance_after, 5_794);
    assert_eq!(
        collat_token_0_total_supply_after,
        pool_collat_token_0_total_supply_after
    );
    assert_eq!(
        debt_token_0_total_supply_after,
        pool_debt_token_0_total_supply_after
    );
    assert_eq!(
        collat_token_2_total_supply_after,
        pool_collat_token_2_total_supply_after
    );
    assert_eq!(
        debt_token_2_total_supply_after,
        pool_debt_token_2_total_supply_after
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
    let reserve_1 = sut
        .pool
        .get_reserve(&sut.reserves[0].token.address)
        .unwrap();
    let reserve_2 = sut
        .pool
        .get_reserve(&sut.reserves[1].token.address)
        .unwrap();
    let reserve_3 = sut
        .pool
        .get_reserve(&sut.reserves[2].token.address)
        .unwrap();

    sut.reserves[0].token_admin.mint(&liquidator, &100_000_000);
    sut.reserves[2].token_admin.mint(&borrower, &100_000_000);

    sut.pool
        .deposit(&liquidator, &debt_config.token.address, &100_000_000);
    sut.pool
        .borrow(&liquidator, &sut.reserves[0].token.address, &20_000_000);
    sut.pool
        .deposit(&borrower, &sut.reserves[2].token.address, &90_000_000);
    sut.price_feed
        .set_price(&token_address, &(10i128.pow(sut.price_feed.decimals()) * 2));

    env.ledger().with_mut(|li| li.timestamp = 5 * DAY);

    let liquidator_user_config = sut.pool.user_configuration(&liquidator);
    let borrower_user_config = sut.pool.user_configuration(&borrower);

    let is_liquidator_borrowed_asset_1_before =
        liquidator_user_config.is_borrowing(&env, reserve_1.get_id());
    let is_liquidator_deposited_asset_1_before =
        liquidator_user_config.is_using_as_collateral(&env, reserve_1.get_id());
    let is_borrower_borrowed_asset_2_before =
        borrower_user_config.is_borrowing(&env, reserve_2.get_id());
    let is_borrower_deposited_asset_1_before =
        borrower_user_config.is_using_as_collateral(&env, reserve_1.get_id());
    let is_borrower_deposited_asset_3_before =
        borrower_user_config.is_using_as_collateral(&env, reserve_3.get_id());

    env.ledger().with_mut(|li| li.timestamp = 6 * DAY);

    sut.pool.liquidate(&liquidator, &borrower, &true);

    env.ledger().with_mut(|li| li.timestamp = 7 * DAY);

    let liquidator_user_config = sut.pool.user_configuration(&liquidator);
    let borrower_user_config = sut.pool.user_configuration(&borrower);

    let is_liquidator_borrowed_asset_1_after =
        liquidator_user_config.is_borrowing(&env, reserve_1.get_id());
    let is_liquidator_deposited_asset_1_after =
        liquidator_user_config.is_using_as_collateral(&env, reserve_1.get_id());
    let is_borrower_borrowed_asset_2_after =
        borrower_user_config.is_borrowing(&env, reserve_2.get_id());
    let is_borrower_deposited_asset_1_after =
        borrower_user_config.is_using_as_collateral(&env, reserve_1.get_id());
    let is_borrower_deposited_asset_3_after =
        borrower_user_config.is_using_as_collateral(&env, reserve_3.get_id());

    assert_eq!(is_liquidator_borrowed_asset_1_before, true);
    assert_eq!(is_liquidator_deposited_asset_1_before, false);
    assert_eq!(is_borrower_borrowed_asset_2_before, true);
    assert_eq!(is_borrower_deposited_asset_1_before, true);
    assert_eq!(is_borrower_deposited_asset_3_before, true);

    assert_eq!(is_liquidator_borrowed_asset_1_after, false);
    assert_eq!(is_liquidator_deposited_asset_1_after, true);
    assert_eq!(is_borrower_borrowed_asset_2_after, false);
    assert_eq!(is_borrower_deposited_asset_1_after, false);
    assert_eq!(is_borrower_deposited_asset_3_after, true);
}

#[test]
fn should_affect_account_data() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, liquidator, _) = fill_pool_three(&env, &sut);

    env.ledger().with_mut(|li| li.timestamp = 4 * DAY);

    let borrower_account_position_before = sut.pool.account_position(&borrower);

    sut.pool.liquidate(&liquidator, &borrower, &true);

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
    let (_, borrower, liquidator, _) = fill_pool_three(&env, &sut);

    env.ledger().with_mut(|li| li.timestamp = 4 * DAY);

    let asset_1 = sut.reserves[0].token.address.clone();
    let asset_2 = sut.reserves[1].token.address.clone();

    let asset_1_collat_coeff_before = sut.pool.collat_coeff(&asset_1);
    let asset_1_debt_coeff_before = sut.pool.debt_coeff(&asset_1);
    let asset_2_collat_coeff_before = sut.pool.collat_coeff(&asset_2);
    let asset_2_debt_coeff_before = sut.pool.debt_coeff(&asset_2);

    env.ledger().with_mut(|li| li.timestamp = 5 * DAY);

    sut.pool.liquidate(&liquidator, &borrower, &false);

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
    let (_, borrower, liquidator, _) = fill_pool_three(&env, &sut);

    env.ledger().with_mut(|li| li.timestamp = 4 * DAY);

    sut.pool.liquidate(&liquidator, &borrower, &false);

    env.ledger().with_mut(|li| li.timestamp = 5 * DAY);

    let mut events = env.events().all();
    let event = events.pop_back_unchecked();

    assert_eq!(
        vec![&env, event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "liquidation"), borrower.clone()).into_val(&env),
                (60_048_996i128, 66_053_895i128).into_val(&env)
            ),
        ]
    );
}
