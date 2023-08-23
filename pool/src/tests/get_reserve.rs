use crate::tests::sut::{
    create_pool_contract, create_s_token_contract, create_token_contract, init_pool,
};
use crate::*;
use soroban_sdk::testutils::Address as _;

#[test]
fn should_be_none_when_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    let uninitialized_asset = Address::random(&env);
    let sut = init_pool(&env, false);

    let reserve = sut.pool.get_reserve(&uninitialized_asset);

    assert!(reserve.is_none());
}

#[test]
fn shoould_return_reserve() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::random(&env);
    let token_admin = Address::random(&env);

    let (underlying_token, _) = create_token_contract(&env, &token_admin);
    let (debt_token, _) = create_token_contract(&env, &token_admin);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, false);
    let s_token = create_s_token_contract(&env, &pool.address, &underlying_token.address);
    assert!(pool.get_reserve(&underlying_token.address).is_none());

    let init_reserve_input = InitReserveInput {
        s_token_address: s_token.address.clone(),
        debt_token_address: debt_token.address.clone(),
    };

    pool.init_reserve(
        &underlying_token.address.clone(),
        &init_reserve_input.clone(),
    );

    let reserve = pool.get_reserve(&underlying_token.address).unwrap();

    assert_eq!(reserve.s_token_address, s_token.address);
    assert_eq!(reserve.debt_token_address, debt_token.address);
}
