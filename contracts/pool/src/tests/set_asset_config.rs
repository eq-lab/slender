#![cfg(test)]
extern crate std;

use soroban_sdk::testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation};
use soroban_sdk::{IntoVal, Symbol};

use crate::tests::sut::init_pool;
use crate::*;

use super::sut::{create_pool_contract, create_token_contract};

#[test]
fn should_require_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let underlying_token = sut.reserves[0].token.address.clone();
    sut.pool.set_asset_config(&underlying_token, &true, &9u32);

    assert_eq!(
        env.auths(),
        [(
            sut.pool_admin.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "set_asset_config"),
                    (underlying_token, true, 9u32).into_val(&env)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #100)")]
fn should_fail_when_reserve_not_exists() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::random(&env);
    let token_admin = Address::random(&env);

    let (underlying_token, _) = create_token_contract(&env, &token_admin);
    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, false);

    pool.set_asset_config(&underlying_token.address, &true, &9u32);
}

#[test]
fn should_set_asset_config() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let underlying_token = sut.reserves[0].token.address.clone();

    sut.pool.set_asset_config(&underlying_token, &true, &9u32);

    let reserve = sut.pool.get_reserve(&underlying_token).unwrap();
    assert_eq!(reserve.configuration.is_base_asset, true);
    assert_eq!(reserve.configuration.decimals, 9u32);

    sut.pool.set_asset_config(&underlying_token, &false, &7u32);

    let reserve = sut.pool.get_reserve(&underlying_token).unwrap();
    assert_eq!(reserve.configuration.is_base_asset, false);
    assert_eq!(reserve.configuration.decimals, 7u32);
}
