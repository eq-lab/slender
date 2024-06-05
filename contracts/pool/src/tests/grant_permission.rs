extern crate std;

use crate::*;
use soroban_sdk::{
    testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation},
    vec, Address, Env, IntoVal, Symbol,
};
use tests::sut::init_pool;

#[test]
fn should_require_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let grant_permission_owner = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &grant_permission_owner,
        &Permission::Permission,
    );

    let perm_receiver = Address::generate(&env);
    sut.pool.grant_permission(
        &grant_permission_owner,
        &perm_receiver,
        &Permission::ClaimProtocolFee,
    );

    assert_eq!(
        env.auths(),
        [(
            grant_permission_owner.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "grant_permission"),
                    vec![
                        &env,
                        grant_permission_owner.into_val(&env),
                        perm_receiver.into_val(&env),
                        Permission::ClaimProtocolFee.into_val(&env)
                    ]
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_no_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::Permission);
    let no_perm = Address::generate(&env);
    let permissioned = sut.pool.permissioned(&Permission::Permission);

    assert!(permissioned.binary_search(&no_perm).is_err());

    let perm_receiver = Address::generate(&env);
    sut.pool
        .grant_permission(&no_perm, &perm_receiver, &Permission::ClaimProtocolFee);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_has_another_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::Permission);
    let another_perm = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &another_perm,
        &Permission::ClaimProtocolFee,
    );
    let permissioned = sut.pool.permissioned(&Permission::Permission);

    assert!(permissioned.binary_search(&another_perm).is_err());

    let perm_receiver = Address::generate(&env);
    sut.pool
        .grant_permission(&another_perm, &perm_receiver, &Permission::ClaimProtocolFee);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_permission_revoked() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::Permission);
    let revoked_perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &revoked_perm, &Permission::Permission);
    sut.pool
        .revoke_permission(&sut.pool_admin, &revoked_perm, &Permission::Permission);
    let permissioned = sut.pool.permissioned(&Permission::Permission);

    assert!(permissioned.binary_search(&revoked_perm).is_err());

    let perm_receiver = Address::generate(&env);
    sut.pool
        .grant_permission(&revoked_perm, &perm_receiver, &Permission::ClaimProtocolFee);
}

#[test]
fn should_grant_permission() {
    let e = Env::default();
    e.mock_all_auths();
    let pool = LendingPoolClient::new(&e, &e.register_contract(None, LendingPool));
    let pool_admin = Address::generate(&e);

    let permissions = [
        Permission::ClaimProtocolFee,
        Permission::CollateralReserveParams,
        Permission::SetReserveBorrowing,
        Permission::InitReserve,
        Permission::SetGracePeriod,
        Permission::SetIRParams,
        Permission::SetPause,
        Permission::SetPoolConfiguration,
        Permission::SetPriceFeeds,
        Permission::SetReserveStatus,
        Permission::UpgradeLPTokens,
        Permission::UpgradePoolWasm,
        Permission::Permission,
    ];

    for p in &permissions {
        assert!(pool.permissioned(p).is_empty());
    }

    pool.initialize(
        &pool_admin,
        &IRParams {
            alpha: 143,
            initial_rate: 200,
            max_rate: 50_000,
            scaling_coeff: 9_000,
        },
        &PoolConfig {
            base_asset_address: Address::generate(&e),
            base_asset_decimals: 7,
            flash_loan_fee: 5,
            initial_health: 0,
            timestamp_window: 20,
            grace_period: 1,
            user_assets_limit: 4,
            min_collat_amount: 0,
            min_debt_amount: 0,
            liquidation_protocol_fee: 0,
        },
    );

    let permissioned_permission = pool.permissioned(&Permission::Permission);
    assert!(permissioned_permission.len() == 1);
    assert!(permissioned_permission.get(0) == Some(pool_admin.clone()));

    for i in 0..permissions.len() {
        let p = permissions.get(i).unwrap();
        let mut expected_permissioned = std::vec![];
        if p == &Permission::Permission {
            expected_permissioned.push(pool_admin.clone());
        }
        for _ in 0..(i + 1) {
            let permission_receiver = Address::generate(&e);
            pool.grant_permission(&pool_admin, &permission_receiver, &p);
            expected_permissioned.push(permission_receiver);
            expected_permissioned.sort(); // check that onchain permissioned addresses are sorted
            let actual_permissioned = std::vec::Vec::from_iter(pool.permissioned(&p).iter());
            assert_eq!(expected_permissioned, actual_permissioned);
        }
    }
}

#[test]
fn should_not_affect_repeated_grant_permission() {
    let e = Env::default();
    e.mock_all_auths();
    let pool = LendingPoolClient::new(&e, &e.register_contract(None, LendingPool));
    let pool_admin = Address::generate(&e);

    pool.initialize(
        &pool_admin,
        &IRParams {
            alpha: 143,
            initial_rate: 200,
            max_rate: 50_000,
            scaling_coeff: 9_000,
        },
        &PoolConfig {
            base_asset_address: Address::generate(&e),
            base_asset_decimals: 7,
            flash_loan_fee: 5,
            initial_health: 0,
            timestamp_window: 20,
            grace_period: 1,
            user_assets_limit: 4,
            min_collat_amount: 0,
            min_debt_amount: 0,
            liquidation_protocol_fee: 0,
        },
    );

    let permission_receiver = Address::generate(&e);
    assert!(pool.permissioned(&Permission::ClaimProtocolFee).is_empty());

    pool.grant_permission(&pool_admin, &pool_admin, &Permission::ClaimProtocolFee);
    let mut expected_permissioned = std::vec![pool_admin.clone(), permission_receiver.clone()];
    expected_permissioned.sort();
    let expected_permissioned = Vec::from_slice(&e, &expected_permissioned);

    for _ in 0..15 {
        pool.grant_permission(
            &pool_admin,
            &permission_receiver,
            &Permission::ClaimProtocolFee,
        );
        assert_eq!(
            expected_permissioned,
            pool.permissioned(&Permission::ClaimProtocolFee)
        );
    }
}
