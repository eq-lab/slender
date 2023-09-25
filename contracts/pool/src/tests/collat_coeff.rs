use crate::tests::sut::{fill_pool_three, init_pool, DAY};
use crate::*;
use soroban_sdk::testutils::{Address as _, Ledger};

#[test]
fn should_update_when_deposit_borrow_withdraw_liquidate() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let debt_token = sut.reserves[1].token.address.clone();
    let deposit_token = sut.reserves[0].token.address.clone();

    let lender = Address::random(&env);
    let borrower = Address::random(&env);

    for r in sut.reserves.iter() {
        r.token_admin.mint(&lender, &1_000_000_000);
        r.token_admin.mint(&borrower, &1_000_000_000);
    }

    env.ledger().with_mut(|l| l.timestamp = DAY);

    for r in sut.reserves.iter() {
        sut.pool.deposit(&lender, &r.token.address, &100_000_000);
    }

    sut.pool.deposit(&borrower, &deposit_token, &100_000_000);
    sut.pool.borrow(&borrower, &debt_token, &40_000_000);

    env.ledger().with_mut(|l| l.timestamp = 2 * DAY);

    let collat_coeff_initial = sut.pool.collat_coeff(&debt_token);

    sut.pool
        .withdraw(&borrower, &deposit_token, &10_000_000, &lender);

    env.ledger().with_mut(|l| l.timestamp = 3 * DAY);
    let collat_coeff_after_withdraw = sut.pool.collat_coeff(&debt_token);

    sut.pool.borrow(&borrower, &debt_token, &10_000_000);

    env.ledger().with_mut(|l| l.timestamp = 4 * DAY);
    let collat_coeff_after_borrow = sut.pool.collat_coeff(&debt_token);

    sut.price_feed.set_price(&debt_token, &1_200_000_000);

    env.ledger().with_mut(|l| l.timestamp = 5 * DAY);
    let collat_coeff_after_price_change = sut.pool.collat_coeff(&debt_token);

    sut.pool.liquidate(&lender, &borrower, &false);

    env.ledger().with_mut(|l| l.timestamp = 6 * DAY);
    let collat_coeff_after_liquidate = sut.pool.collat_coeff(&debt_token);

    assert_eq!(collat_coeff_initial, 1_000_000_000);
    assert_eq!(collat_coeff_after_withdraw, 1_000_000_000);
    assert_eq!(collat_coeff_after_borrow, 1_000_199_480);
    assert_eq!(collat_coeff_after_price_change, 1_000_265_990);
    assert_eq!(collat_coeff_after_liquidate, 1_000_295_540);
}

#[test]
fn should_change_over_time() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, _, _, debt_config) = fill_pool_three(&env, &sut);
    let debt_token = debt_config.token.address.clone();

    let collat_coeff_1 = sut.pool.collat_coeff(&debt_token);

    env.ledger().with_mut(|l| l.timestamp = 4 * DAY);
    let collat_coeff_2 = sut.pool.collat_coeff(&debt_token);

    env.ledger().with_mut(|l| l.timestamp = 5 * DAY);
    let collat_coeff_3 = sut.pool.collat_coeff(&debt_token);

    assert_eq!(collat_coeff_1, 1_000_330_690);
    assert_eq!(collat_coeff_2, 1_000_440_950);
    assert_eq!(collat_coeff_3, 1_000_551_210);
}

#[test]
fn should_change_when_elapsed_time_gte_window() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    sut.pool.set_reserve_timestamp_window(&20);

    let (_, _, _, debt_config) = fill_pool_three(&env, &sut);
    let debt_token = debt_config.token.address.clone();

    let collat_coeff_1 = sut.pool.collat_coeff(&debt_token);

    env.ledger().with_mut(|li| li.timestamp = 3 * DAY + 19);

    let collat_coeff_2 = sut.pool.collat_coeff(&debt_token);

    env.ledger().with_mut(|li| li.timestamp = 3 * DAY + 26);

    let collat_coeff_3 = sut.pool.collat_coeff(&debt_token);

    env.ledger().with_mut(|li| li.timestamp = 3 * DAY + 51);

    let collat_coeff_4 = sut.pool.collat_coeff(&debt_token);

    env.ledger().with_mut(|li| li.timestamp = 3 * DAY + 55);

    let collat_coeff_5 = sut.pool.collat_coeff(&debt_token);

    env.ledger().with_mut(|li| li.timestamp = 3 * DAY + 61);

    let collat_coeff_6 = sut.pool.collat_coeff(&debt_token);

    assert_eq!(collat_coeff_1, collat_coeff_2);
    assert!(collat_coeff_3 > collat_coeff_2);
    assert!(collat_coeff_4 > collat_coeff_3);
    assert_eq!(collat_coeff_4, collat_coeff_5);
    assert!(collat_coeff_6 > collat_coeff_5);
}
