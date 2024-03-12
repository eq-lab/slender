#![cfg(test)]
extern crate std;

use crate::tests::sut::{
    create_pool_contract, create_s_token_contract, create_token_contract, init_pool,
};
use crate::*;
use pool_interface::types::reserve_type::ReserveType;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation};
use soroban_sdk::{IntoVal, Symbol};

#[test]
fn should_require_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (underlying_token, _) = create_token_contract(&env, &token_admin);
    let (debt_token, _) = create_token_contract(&env, &token_admin);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, false);
    let s_token = create_s_token_contract(&env, &pool.address, &underlying_token.address);
    assert!(pool.get_reserve(&underlying_token.address).is_none());

    let init_reserve_input =
        ReserveType::Fungible(s_token.address.clone(), debt_token.address.clone());

    pool.init_reserve(
        &underlying_token.address.clone(),
        &init_reserve_input.clone(),
    );

    assert_eq!(
        env.auths(),
        [(
            admin.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    pool.address.clone(),
                    Symbol::new(&env, "init_reserve"),
                    (
                        underlying_token.address.clone(),
                        // false,
                        init_reserve_input.clone()
                    )
                        .into_val(&env)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #105)")]
fn should_fail_when_calling_second_time() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let init_reserve_input = ReserveType::Fungible(
        sut.s_token().address.clone(),
        sut.debt_token().address.clone(),
    );

    sut.pool
        .init_reserve(&sut.token().address, &init_reserve_input);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn should_fail_when_pool_not_initialized() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);

    let (underlying_token, _) = create_token_contract(&env, &token_admin);
    let (debt_token, _) = create_token_contract(&env, &token_admin);

    let pool: LendingPoolClient<'_> =
        LendingPoolClient::new(&env, &env.register_contract(None, LendingPool));
    let s_token = create_s_token_contract(&env, &pool.address, &underlying_token.address);
    assert!(pool.get_reserve(&underlying_token.address).is_none());

    let init_reserve_input =
        ReserveType::Fungible(s_token.address.clone(), debt_token.address.clone());

    pool.init_reserve(&underlying_token.address, &init_reserve_input);
}

#[test]
fn should_set_underlying_asset_s_token_and_debt_token_addresses() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (underlying_token, _) = create_token_contract(&env, &token_admin);
    let (debt_token, _) = create_token_contract(&env, &token_admin);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, false);
    let s_token = create_s_token_contract(&env, &pool.address, &underlying_token.address);
    assert!(pool.get_reserve(&underlying_token.address).is_none());

    let init_reserve_input =
        ReserveType::Fungible(s_token.address.clone(), debt_token.address.clone());

    pool.init_reserve(
        &underlying_token.address.clone(),
        &init_reserve_input.clone(),
    );
    assert!(pool.get_reserve(&underlying_token.address).is_some());

    let reserve = pool.get_reserve(&underlying_token.address).unwrap();

    assert_eq!(reserve.reserve_type, init_reserve_input);
}
