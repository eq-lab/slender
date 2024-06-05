use crate::methods::utils::rate::calc_next_accrued_rate;
use crate::tests::sut::{fill_pool, fill_pool_three, init_pool, DAY};
use crate::*;
use common::FixedI128;
use price_feed_interface::types::asset::Asset;
use price_feed_interface::types::price_data::PriceData;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::vec;
use tests::sut::set_time;

#[test]
fn should_update_when_deposit_borrow_withdraw_liquidate() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    sut.pool.set_pool_configuration(
        &sut.pool_admin,
        &PoolConfig {
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
        },
    );

    let debt_token = sut.reserves[1].token.address.clone();
    let deposit_token = sut.reserves[0].token.address.clone();

    let lender = Address::generate(&env);
    let borrower = Address::generate(&env);

    for i in 0..3 {
        let amount = (i == 0).then(|| 10_000_000).unwrap_or(1_000_000_000);

        sut.reserves[i].token_admin.mint(&lender, &amount);
        sut.reserves[i].token_admin.mint(&borrower, &amount);
    }

    set_time(&env, &sut, DAY, false);

    for i in 0..3 {
        let amount = (i == 0).then(|| 1_000_000).unwrap_or(100_000_000);

        sut.pool
            .deposit(&lender, &sut.reserves[i].token.address, &amount);
    }

    sut.pool.deposit(&borrower, &deposit_token, &1_000_000);
    sut.pool.borrow(&borrower, &debt_token, &40_000_000);

    set_time(&env, &sut, 2 * DAY, false);

    let debt_coeff_initial = sut.pool.debt_coeff(&debt_token);

    sut.pool
        .withdraw(&borrower, &deposit_token, &100_000, &lender);

    set_time(&env, &sut, 3 * DAY, false);
    let debt_coeff_after_withdraw = sut.pool.debt_coeff(&debt_token);

    sut.pool.borrow(&borrower, &debt_token, &400_000);

    set_time(&env, &sut, 4 * DAY, false);
    let debt_coeff_after_borrow = sut.pool.debt_coeff(&debt_token);

    sut.price_feed.init(
        &Asset::Stellar(debt_token.clone()),
        &vec![
            &env,
            PriceData {
                price: 14_000_000_000_000_000,
                timestamp: 5 * DAY,
            },
        ],
    );

    set_time(&env, &sut, 5 * DAY, false);
    let debt_coeff_after_price_change = sut.pool.debt_coeff(&debt_token);

    sut.pool.liquidate(&lender, &borrower);

    set_time(&env, &sut, 6 * DAY, false);
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

    set_time(&env, &sut, 4 * DAY, false);
    let debt_coeff_2 = sut.pool.debt_coeff(&debt_token);

    set_time(&env, &sut, 5 * DAY, false);
    let debt_coeff_3 = sut.pool.debt_coeff(&debt_token);

    assert_eq!(debt_coeff_1, 1_000_609_032);
    assert_eq!(debt_coeff_2, 1_000_812_113);
    assert_eq!(debt_coeff_3, 1_001_015_195);
}

#[test]
fn should_be_correctly_calculated() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let (_, borrower, debt_config) = fill_pool(&env, &sut, true);

    set_time(&env, &sut, 2 * DAY, false);

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
    set_time(&env, &sut, 10 * DAY, false);

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
    sut.pool.set_pool_configuration(
        &sut.pool_admin,
        &PoolConfig {
            base_asset_address: sut.reserves[0].token.address.clone(),
            base_asset_decimals: sut.reserves[0].token.decimals(),
            flash_loan_fee: 5,
            initial_health: 0,
            timestamp_window: 20,
            grace_period: 1,
            user_assets_limit: 4,
            min_collat_amount: 0,
            min_debt_amount: 0,
            liquidation_protocol_fee: 0,
        },
    );

    let (_, _, _, debt_config) = fill_pool_three(&env, &sut);
    let debt_token = debt_config.token.address.clone();

    let debt_coeff_1 = sut.pool.debt_coeff(&debt_token);

    set_time(&env, &sut, 3 * DAY + 19, false);

    let debt_coeff_2 = sut.pool.debt_coeff(&debt_token);

    set_time(&env, &sut, 3 * DAY + 26, false);

    let debt_coeff_3 = sut.pool.debt_coeff(&debt_token);

    set_time(&env, &sut, 3 * DAY + 51, false);

    let debt_coeff_4 = sut.pool.debt_coeff(&debt_token);

    set_time(&env, &sut, 3 * DAY + 55, false);

    let debt_coeff_5 = sut.pool.debt_coeff(&debt_token);

    set_time(&env, &sut, 3 * DAY + 61, false);

    let debt_coeff_6 = sut.pool.debt_coeff(&debt_token);

    assert_eq!(debt_coeff_1, debt_coeff_2);
    assert!(debt_coeff_3 > debt_coeff_2);
    assert!(debt_coeff_4 > debt_coeff_3);
    assert_eq!(debt_coeff_4, debt_coeff_5);
    assert!(debt_coeff_6 > debt_coeff_5);
}
