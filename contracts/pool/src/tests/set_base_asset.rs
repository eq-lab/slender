#![cfg(test)]
extern crate std;

use soroban_sdk::testutils::{AuthorizedFunction, AuthorizedInvocation};
use soroban_sdk::{IntoVal, Symbol};

use crate::tests::sut::init_pool;
use crate::*;

#[test]
fn should_require_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let underlying_token = sut.reserves[0].token.address.clone();
    sut.pool.set_base_asset(&underlying_token, &9u32);

    assert_eq!(
        env.auths(),
        [(
            sut.pool_admin.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "set_base_asset"),
                    (underlying_token, 9u32).into_val(&env)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
}

#[test]
fn should_set_base_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let underlying_token_1 = sut.reserves[0].token.address.clone();
    let underlying_token_2 = sut.reserves[1].token.address.clone();

    let base_asset_init = sut.pool.base_asset();

    assert_eq!(base_asset_init.address, underlying_token_1);
    assert_eq!(base_asset_init.decimals, 7u32);

    sut.pool.set_base_asset(&underlying_token_2, &9u32);
    let base_asset_after = sut.pool.base_asset();

    assert_eq!(base_asset_after.address, underlying_token_2);
    assert_eq!(base_asset_after.decimals, 9u32);
}
