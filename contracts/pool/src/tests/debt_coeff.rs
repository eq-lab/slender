use crate::methods::utils::rate::calc_next_accrued_rate;
use crate::tests::sut::{fill_pool, fill_pool_three, init_pool, DAY};
use crate::*;
use common::FixedI128;
use price_feed_interface::types::asset::Asset;
use price_feed_interface::types::price_data::PriceData;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::vec;

#[test]
fn should_update_when_deposit_borrow_withdraw_liquidate() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    sut.pool.set_initial_health(&2_500);

    let debt_token = sut.reserves[1].token.address.clone();
    let deposit_token = sut.reserves[0].token.address.clone();

    let lender = Address::generate(&env);
    let borrower = Address::generate(&env);

    for i in 0..3 {
        let amount = (i == 0).then(|| 10_000_000).unwrap_or(1_000_000_000);

        sut.reserves[i].token_admin.mint(&lender, &amount);
        sut.reserves[i].token_admin.mint(&borrower, &amount);
    }

    env.ledger().with_mut(|l| l.timestamp = DAY);

    for i in 0..3 {
        let amount = (i == 0).then(|| 1_000_000).unwrap_or(100_000_000);

        sut.pool
            .deposit(&lender, &sut.reserves[i].token.address, &amount);
    }

    sut.pool.deposit(&borrower, &deposit_token, &1_000_000);
    sut.pool.borrow(&borrower, &debt_token, &40_000_000);

    env.ledger().with_mut(|l| l.timestamp = 2 * DAY);

    let debt_coeff_initial = sut.pool.debt_coeff(&debt_token);

    sut.pool
        .withdraw(&borrower, &deposit_token, &100_000, &lender);

    env.ledger().with_mut(|l| l.timestamp = 3 * DAY);
    let debt_coeff_after_withdraw = sut.pool.debt_coeff(&debt_token);

    sut.pool.borrow(&borrower, &debt_token, &400_000);

    env.ledger().with_mut(|l| l.timestamp = 4 * DAY);
    let debt_coeff_after_borrow = sut.pool.debt_coeff(&debt_token);

    sut.price_feed.init(
        &Asset::Stellar(debt_token.clone()),
        &vec![
            &env,
            PriceData {
                price: 14_000_000_000_000_000,
                timestamp: 0,
            },
        ],
    );

    env.ledger().with_mut(|l| l.timestamp = 5 * DAY);
    let debt_coeff_after_price_change = sut.pool.debt_coeff(&debt_token);

    sut.pool.liquidate(&lender, &borrower, &false);

    env.ledger().with_mut(|l| l.timestamp = 6 * DAY);
    let debt_coeff_after_liquidate = sut.pool.debt_coeff(&debt_token);

    assert_eq!(debt_coeff_initial, 1_000_000_000);
    assert_eq!(debt_coeff_after_withdraw, 1_000_000_000);
    assert_eq!(debt_coeff_after_borrow, 1_000_344_464);
    assert_eq!(debt_coeff_after_price_change, 1_000_459_304);
    assert_eq!(debt_coeff_after_liquidate, 1_000_450_253);
}

#[test]
fn should_change_over_time() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, _, _, debt_config) = fill_pool_three(&env, &sut);
    let debt_token = debt_config.token.address.clone();

    let debt_coeff_1 = sut.pool.debt_coeff(&debt_token);

    env.ledger().with_mut(|l| l.timestamp = 4 * DAY);
    let debt_coeff_2 = sut.pool.debt_coeff(&debt_token);

    env.ledger().with_mut(|l| l.timestamp = 5 * DAY);
    let debt_coeff_3 = sut.pool.debt_coeff(&debt_token);

    assert_eq!(debt_coeff_1, 1_000_609_079);
    assert_eq!(debt_coeff_2, 1_000_812_160);
    assert_eq!(debt_coeff_3, 1_001_015_242);
}

#[test]
fn should_be_correctly_calculated() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let (_, borrower, debt_config) = fill_pool(&env, &sut, true);

    env.ledger().with_mut(|l| l.timestamp = 2 * DAY);

    sut.pool
        .borrow(&borrower, &debt_config.token.address, &50_000);
    let reserve = sut.pool.get_reserve(&debt_config.token.address).unwrap();

    let collat_ar = FixedI128::from_inner(reserve.lender_ar);
    let s_token_supply = debt_config.s_token().total_supply();
    let balance = debt_config.token.balance(&debt_config.s_token().address);
    let debt_token_suply = debt_config.debt_token().total_supply();

    let expected_collat_coeff = FixedI128::from_rational(
        balance + collat_ar.mul_int(debt_token_suply).unwrap(),
        s_token_supply,
    )
    .unwrap()
    .into_inner();

    let collat_coeff = sut.pool.collat_coeff(&debt_config.token.address);
    assert_eq!(collat_coeff, expected_collat_coeff);

    // shift time to 8 days
    env.ledger().with_mut(|l| l.timestamp = 10 * DAY);

    let elapsed_time = 8 * DAY;
    let collat_ar = calc_next_accrued_rate(
        collat_ar,
        FixedI128::from_inner(reserve.lender_ir),
        elapsed_time,
    )
    .unwrap();
    let expected_collat_coeff = FixedI128::from_rational(
        balance + collat_ar.mul_int(debt_token_suply).unwrap(),
        s_token_supply,
    )
    .unwrap()
    .into_inner();

    let collat_coeff = sut.pool.collat_coeff(&debt_config.token.address);
    assert_eq!(collat_coeff, expected_collat_coeff);
}

#[test]
fn should_change_when_elapsed_time_gte_window() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    sut.pool.set_reserve_timestamp_window(&20);

    let (_, _, _, debt_config) = fill_pool_three(&env, &sut);
    let debt_token = debt_config.token.address.clone();

    let debt_coeff_1 = sut.pool.debt_coeff(&debt_token);

    env.ledger().with_mut(|li| li.timestamp = 3 * DAY + 19);

    let debt_coeff_2 = sut.pool.debt_coeff(&debt_token);

    env.ledger().with_mut(|li| li.timestamp = 3 * DAY + 26);

    let debt_coeff_3 = sut.pool.debt_coeff(&debt_token);

    env.ledger().with_mut(|li| li.timestamp = 3 * DAY + 51);

    let debt_coeff_4 = sut.pool.debt_coeff(&debt_token);

    env.ledger().with_mut(|li| li.timestamp = 3 * DAY + 55);

    let debt_coeff_5 = sut.pool.debt_coeff(&debt_token);

    env.ledger().with_mut(|li| li.timestamp = 3 * DAY + 61);

    let debt_coeff_6 = sut.pool.debt_coeff(&debt_token);

    assert_eq!(debt_coeff_1, debt_coeff_2);
    assert!(debt_coeff_3 > debt_coeff_2);
    assert!(debt_coeff_4 > debt_coeff_3);
    assert_eq!(debt_coeff_4, debt_coeff_5);
    assert!(debt_coeff_6 > debt_coeff_5);
}
