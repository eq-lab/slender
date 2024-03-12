use super::sut::fill_pool_three;
use crate::tests::sut::init_pool;
use crate::*;
use price_feed_interface::types::asset::Asset;
use price_feed_interface::types::price_data::PriceData;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::vec;

#[test]
#[should_panic(expected = "HostError: Error(Contract, #202)")]
fn should_fail_when_user_config_not_exist() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, _, liquidator, _) = fill_pool_three(&env, &sut);

    sut.pool.account_position(&liquidator);
}

#[test]
fn should_update_when_deposit_borrow_withdraw_liquidate_price_change() {
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

    for i in 0..3 {
        let amount = (i == 0).then(|| 1_000_000).unwrap_or(100_000_000);

        sut.pool
            .deposit(&lender, &sut.reserves[i].token.address, &amount);
    }

    sut.pool.deposit(&borrower, &deposit_token, &1_000_000); // 100_000_000
    let position_after_deposit = sut.pool.account_position(&borrower);

    sut.pool.borrow(&borrower, &debt_token, &40_000_000);
    let position_after_borrow = sut.pool.account_position(&borrower);

    sut.pool
        .withdraw(&borrower, &deposit_token, &100_000, &lender);
    let position_after_withdraw = sut.pool.account_position(&borrower);

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

    let position_after_change_price = sut.pool.account_position(&borrower);

    sut.pool.liquidate(&lender, &borrower, &false);
    let position_after_liquidate = sut.pool.account_position(&borrower);

    assert_eq!(position_after_deposit.discounted_collateral, 600_000);
    assert_eq!(position_after_deposit.debt, 0);
    assert_eq!(position_after_deposit.npv, 600_000);

    assert_eq!(position_after_borrow.discounted_collateral, 600_000);
    assert_eq!(position_after_borrow.debt, 400_000);
    assert_eq!(position_after_borrow.npv, 200_000);

    assert_eq!(position_after_withdraw.discounted_collateral, 540_000);
    assert_eq!(position_after_withdraw.debt, 400_000);
    assert_eq!(position_after_withdraw.npv, 140_000);

    assert_eq!(position_after_change_price.discounted_collateral, 540_000);
    assert_eq!(position_after_change_price.debt, 560_000);
    assert_eq!(position_after_change_price.npv, -20_000);

    assert_eq!(position_after_liquidate.discounted_collateral, 358_700);
    assert_eq!(position_after_liquidate.debt, 269_026);
    assert_eq!(position_after_liquidate.npv, 89_674);
}
