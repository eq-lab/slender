#![cfg(test)]
extern crate std;

use soroban_sdk::testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation};
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
fn should_require_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let pool_v2_wasm = env.deployer().upload_contract_wasm(pool_v2::WASM);

    let upgrade_owner = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &upgrade_owner,
        &Permission::UpgradePoolWasm,
    );

    sut.pool.upgrade(&upgrade_owner, &pool_v2_wasm);

    assert_eq!(
        env.auths(),
        [(
            upgrade_owner.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    symbol_short!("upgrade"),
                    vec![
                        &env,
                        upgrade_owner.into_val(&env),
                        pool_v2_wasm.into_val(&env)
                    ]
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

    sut.pool
        .upgrade_s_token(&sut.pool_admin, &asset, &s_token_v2_wasm);
    sut.pool
        .upgrade_debt_token(&sut.pool_admin, &asset, &debt_token_v2_wasm);
    sut.pool.upgrade(&sut.pool_admin, &pool_v2_wasm);

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

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_no_permission_upgrade_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    let pool_v2_wasm = env.deployer().upload_contract_wasm(pool_v2::WASM);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::UpgradePoolWasm);
    let no_perm = Address::generate(&env);
    let permissioned = sut.pool.permissioned(&Permission::UpgradePoolWasm);

    assert!(permissioned.binary_search(&no_perm).is_err());

    sut.pool.upgrade(&no_perm, &pool_v2_wasm);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_no_permission_upgrade_s_token() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let asset = sut.reserves[0].token.address.clone();

    let s_token_v2_wasm = env.deployer().upload_contract_wasm(s_token_v2::WASM);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::UpgradeLPTokens);
    let no_perm = Address::generate(&env);
    let permissioned = sut.pool.permissioned(&Permission::UpgradeLPTokens);

    assert!(permissioned.binary_search(&no_perm).is_err());

    sut.pool.upgrade_s_token(&no_perm, &asset, &s_token_v2_wasm);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_no_permission_upgrade_debt_token() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let asset = sut.reserves[0].token.address.clone();

    let debt_token_v2_wasm = env.deployer().upload_contract_wasm(debt_token_v2::WASM);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::UpgradeLPTokens);
    let no_perm = Address::generate(&env);
    let permissioned = sut.pool.permissioned(&Permission::UpgradeLPTokens);

    assert!(permissioned.binary_search(&no_perm).is_err());

    sut.pool
        .upgrade_debt_token(&no_perm, &asset, &debt_token_v2_wasm);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_has_another_permission_upgrade_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    let pool_v2_wasm = env.deployer().upload_contract_wasm(pool_v2::WASM);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::UpgradePoolWasm);
    let another_perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &another_perm, &Permission::UpgradeLPTokens);
    let permissioned = sut.pool.permissioned(&Permission::UpgradePoolWasm);

    assert!(permissioned.binary_search(&another_perm).is_err());

    sut.pool.upgrade(&another_perm, &pool_v2_wasm);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_has_another_permission_upgrade_s_token() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::UpgradeLPTokens);
    let another_perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &another_perm, &Permission::UpgradePoolWasm);
    let permissioned = sut.pool.permissioned(&Permission::UpgradeLPTokens);

    assert!(permissioned.binary_search(&another_perm).is_err());

    let s_token_v2_wasm = env.deployer().upload_contract_wasm(s_token_v2::WASM);
    let asset = sut.reserves[0].token.address.clone();
    sut.pool
        .upgrade_s_token(&another_perm, &asset, &s_token_v2_wasm);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_has_another_permission_upgrade_debt_token() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::UpgradeLPTokens);
    let another_perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &another_perm, &Permission::UpgradePoolWasm);
    let permissioned = sut.pool.permissioned(&Permission::UpgradeLPTokens);

    assert!(permissioned.binary_search(&another_perm).is_err());

    let asset = sut.reserves[0].token.address.clone();

    let debt_token_v2_wasm = env.deployer().upload_contract_wasm(debt_token_v2::WASM);
    sut.pool
        .upgrade_debt_token(&another_perm, &asset, &debt_token_v2_wasm);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_permission_revoked_upgrade_pool() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::UpgradePoolWasm);
    let revoked_perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &revoked_perm, &Permission::UpgradePoolWasm);
    sut.pool
        .revoke_permission(&sut.pool_admin, &revoked_perm, &Permission::UpgradePoolWasm);
    let permissioned = sut.pool.permissioned(&Permission::UpgradePoolWasm);

    assert!(permissioned.binary_search(&revoked_perm).is_err());

    let pool_v2_wasm = env.deployer().upload_contract_wasm(pool_v2::WASM);
    sut.pool.upgrade(&revoked_perm, &pool_v2_wasm);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_permission_revoked_upgrade_s_token() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::UpgradeLPTokens);
    let revoked_perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &revoked_perm, &Permission::UpgradeLPTokens);
    sut.pool
        .revoke_permission(&sut.pool_admin, &revoked_perm, &Permission::UpgradeLPTokens);
    let permissioned = sut.pool.permissioned(&Permission::UpgradeLPTokens);

    assert!(permissioned.binary_search(&revoked_perm).is_err());

    let s_token_v2_wasm = env.deployer().upload_contract_wasm(s_token_v2::WASM);
    let asset = sut.reserves[0].token.address.clone();
    sut.pool
        .upgrade_s_token(&revoked_perm, &asset, &s_token_v2_wasm);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_permission_revoked_upgrade_debt_token() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::UpgradeLPTokens);
    let revoked_perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &revoked_perm, &Permission::UpgradeLPTokens);
    sut.pool
        .revoke_permission(&sut.pool_admin, &revoked_perm, &Permission::UpgradeLPTokens);
    let permissioned = sut.pool.permissioned(&Permission::UpgradeLPTokens);

    assert!(permissioned.binary_search(&revoked_perm).is_err());

    let debt_token_v2_wasm = env.deployer().upload_contract_wasm(debt_token_v2::WASM);
    let asset = sut.reserves[0].token.address.clone();
    sut.pool
        .upgrade_debt_token(&revoked_perm, &asset, &debt_token_v2_wasm);
}
