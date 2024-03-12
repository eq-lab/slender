#![cfg(test)]
extern crate std;

use soroban_sdk::testutils::{AuthorizedFunction, AuthorizedInvocation};
use soroban_sdk::{symbol_short, vec, IntoVal};

use crate::tests::sut::init_pool;
use crate::*;

pub mod pool_v2 {
    soroban_sdk::contractimport!(file = "../../mocks/pool_v2_mock.wasm");
}

pub mod s_token_v2 {
    soroban_sdk::contractimport!(file = "../../mocks/s_token_v2_mock.wasm");
}

pub mod debt_token_v2 {
    soroban_sdk::contractimport!(file = "../../mocks/debt_token_v2_mock.wasm");
}

#[test]
fn should_require_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let pool_v2_wasm = env.deployer().upload_contract_wasm(pool_v2::WASM);

    sut.pool.upgrade(&pool_v2_wasm);

    assert_eq!(
        env.auths(),
        [(
            sut.pool_admin,
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    symbol_short!("upgrade"),
                    vec![&env, pool_v2_wasm.into_val(&env)]
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
}

#[test]
fn should_upgrade_contracts() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let asset = sut.reserves[0].token.address.clone();

    let pool_v2_wasm = env.deployer().upload_contract_wasm(pool_v2::WASM);
    let s_token_v2_wasm = env.deployer().upload_contract_wasm(s_token_v2::WASM);
    let debt_token_v2_wasm = env.deployer().upload_contract_wasm(debt_token_v2::WASM);

    let pool_version_before = sut.pool.version();
    let s_token_version_before = sut.s_token().version();
    let debt_token_version_before = sut.debt_token().version();

    sut.pool.upgrade_s_token(&asset, &s_token_v2_wasm);
    sut.pool.upgrade_debt_token(&asset, &debt_token_v2_wasm);
    sut.pool.upgrade(&pool_v2_wasm);

    let pool_version_after = sut.pool.version();
    let s_token_version_after = sut.s_token().version();
    let debt_token_version_after = sut.debt_token().version();

    assert_eq!(pool_version_before, 1);
    assert_eq!(pool_version_after, 2);
    assert_eq!(s_token_version_before, 1);
    assert_eq!(s_token_version_after, 2);
    assert_eq!(debt_token_version_before, 1);
    assert_eq!(debt_token_version_after, 2);
}
