use super::sut::fill_pool_three;
use crate::tests::sut::init_pool;
use crate::*;
use soroban_sdk::testutils::Address as _;

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_user_config_not_exist() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, _, liquidator, _) = fill_pool_three(&env, &sut);

    sut.pool.account_position(&liquidator);

    // assert_eq!(
    //     sut.pool
    //         .try_account_position(&liquidator)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::UserConfigNotExists
    // )
}

#[test]
fn should_update_when_deposit_borrow_withdraw_liquidate_price_change() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let debt_token = sut.reserves[1].token.address.clone();
    let deposit_token = sut.reserves[0].token.address.clone();

    let lender = Address::random(&env);
    let borrower = Address::random(&env);

    for r in sut.reserves.iter() {
        r.token_admin.mint(&lender, &1_000_000_000);
        r.token_admin.mint(&borrower, &1_000_000_000);
    }

    for r in sut.reserves.iter() {
        sut.pool.deposit(&lender, &r.token.address, &100_000_000);
    }

    sut.pool.deposit(&borrower, &deposit_token, &100_000_000);
    let position_after_deposit = sut.pool.account_position(&borrower);

    sut.pool.borrow(&borrower, &debt_token, &40_000_000);
    let position_after_borrow = sut.pool.account_position(&borrower);

    sut.pool
        .withdraw(&borrower, &deposit_token, &10_000_000, &lender);
    let position_after_withdraw = sut.pool.account_position(&borrower);

    sut.price_feed.set_price(&debt_token, &1_400_000_000);
    let position_after_change_price = sut.pool.account_position(&borrower);

    sut.pool.liquidate(&lender, &borrower, &false);
    let position_after_liquidate = sut.pool.account_position(&borrower);

    assert_eq!(position_after_deposit.discounted_collateral, 60_000_000);
    assert_eq!(position_after_deposit.debt, 0);
    assert_eq!(position_after_deposit.npv, 60_000_000);

    assert_eq!(position_after_borrow.discounted_collateral, 60_000_000);
    assert_eq!(position_after_borrow.debt, 40_000_000);
    assert_eq!(position_after_borrow.npv, 20_000_000);

    assert_eq!(position_after_withdraw.discounted_collateral, 54_000_000);
    assert_eq!(position_after_withdraw.debt, 40_000_000);
    assert_eq!(position_after_withdraw.npv, 14_000_000);

    assert_eq!(
        position_after_change_price.discounted_collateral,
        54_000_000
    );
    assert_eq!(position_after_change_price.debt, 56_000_000);
    assert_eq!(position_after_change_price.npv, -2_000_000);

    assert_eq!(position_after_liquidate.discounted_collateral, 17_040_000);
    assert_eq!(position_after_liquidate.debt, 0);
    assert_eq!(position_after_liquidate.npv, 17_040_000);
}
