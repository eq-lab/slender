use super::sut::fill_pool_three;
use crate::tests::sut::init_pool;
use crate::*;

#[test]
#[should_panic(expected = "HostError: Error(Contract, #202)")]
fn should_fail_when_user_config_not_exist() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, _, liquidator, _) = fill_pool_three(&env, &sut);

    sut.pool.user_configuration(&liquidator);
}

#[test]
fn should_return_user_config() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _, debt_config) = fill_pool_three(&env, &sut);
    let debt_address = debt_config.token.address.clone();
    let collat_address = sut.reserves[0].token.address.clone();
    let debt_reserve_id = sut.pool.get_reserve(&debt_address).unwrap().get_id();
    let collat_reserve_id = sut.pool.get_reserve(&collat_address).unwrap().get_id();

    let borrower_user_config = sut.pool.user_configuration(&borrower);

    assert_eq!(borrower_user_config.is_borrowing_any(), true);
    assert_eq!(
        borrower_user_config.is_borrowing(&env, debt_reserve_id),
        true
    );
    assert_eq!(
        borrower_user_config.is_borrowing(&env, collat_reserve_id),
        false
    );
    assert_eq!(
        borrower_user_config.is_using_as_collateral(&env, debt_reserve_id),
        false
    );
    assert_eq!(
        borrower_user_config.is_using_as_collateral(&env, collat_reserve_id),
        true
    );
    assert_eq!(
        borrower_user_config.is_using_as_collateral_or_borrowing(&env, debt_reserve_id),
        true
    );
    assert_eq!(
        borrower_user_config.is_using_as_collateral_or_borrowing(&env, collat_reserve_id),
        true
    );
}
