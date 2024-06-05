#![cfg(test)]
extern crate std;

use crate::tests::sut::init_pool;
use crate::*;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation};
use soroban_sdk::{vec, IntoVal, Symbol};

#[test]
fn should_require_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let ir_params_input = IRParams {
        alpha: 144,
        initial_rate: 201,
        max_rate: 50_001,
        scaling_coeff: 9_001,
    };

    let set_ir_params_owner = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &set_ir_params_owner,
        &Permission::SetIRParams,
    );

    sut.pool
        .set_ir_params(&set_ir_params_owner, &ir_params_input.clone());

    assert_eq!(
        env.auths(),
        [(
            set_ir_params_owner.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    sut.pool.address.clone(),
                    Symbol::new(&env, "set_ir_params"),
                    vec![
                        &env,
                        set_ir_params_owner.into_val(&env),
                        ir_params_input.into_val(&env)
                    ]
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #401)")]
fn should_fail_when_invalid_initial_rate() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let ir_params_input = IRParams {
        alpha: 144,
        initial_rate: 10001,
        max_rate: 50_001,
        scaling_coeff: 9_001,
    };

    sut.pool
        .set_ir_params(&sut.pool_admin, &ir_params_input.clone());
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #403)")]
fn should_fail_when_invalid_max_rate() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let ir_params_input = IRParams {
        alpha: 144,
        initial_rate: 201,
        max_rate: 10_000,
        scaling_coeff: 9_001,
    };

    sut.pool
        .set_ir_params(&sut.pool_admin, &ir_params_input.clone());
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #402)")]
fn should_fail_when_invalid_scaling_coeff() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let ir_params_input = IRParams {
        alpha: 144,
        initial_rate: 201,
        max_rate: 50_001,
        scaling_coeff: 10_000,
    };

    sut.pool
        .set_ir_params(&sut.pool_admin, &ir_params_input.clone());
}

#[test]
fn should_set_ir_params() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let ir_params_input = IRParams {
        alpha: 144,
        initial_rate: 201,
        max_rate: 50_001,
        scaling_coeff: 9_001,
    };

    sut.pool.set_ir_params(&sut.pool_admin, &ir_params_input);

    let ir_params = sut.pool.ir_params().unwrap();

    assert_eq!(ir_params_input.alpha, ir_params.alpha);
    assert_eq!(ir_params_input.initial_rate, ir_params.initial_rate);
    assert_eq!(ir_params_input.max_rate, ir_params.max_rate);
    assert_eq!(ir_params_input.scaling_coeff, ir_params.scaling_coeff);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_no_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let ir_params_input = IRParams {
        alpha: 144,
        initial_rate: 201,
        max_rate: 50_001,
        scaling_coeff: 9_001,
    };

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetIRParams);
    let no_perm = Address::generate(&env);
    let permissioned = sut.pool.permissioned(&Permission::SetIRParams);

    assert!(permissioned.binary_search(&no_perm).is_err());

    sut.pool.set_ir_params(&no_perm, &ir_params_input);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_has_another_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let ir_params_input = IRParams {
        alpha: 144,
        initial_rate: 201,
        max_rate: 50_001,
        scaling_coeff: 9_001,
    };

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetIRParams);
    let another_perm = Address::generate(&env);
    sut.pool.grant_permission(
        &sut.pool_admin,
        &another_perm,
        &Permission::ClaimProtocolFee,
    );
    let permissioned = sut.pool.permissioned(&Permission::SetIRParams);

    assert!(permissioned.binary_search(&another_perm).is_err());

    sut.pool.set_ir_params(&another_perm, &ir_params_input);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_permission_revoked() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let ir_params_input = IRParams {
        alpha: 144,
        initial_rate: 201,
        max_rate: 50_001,
        scaling_coeff: 9_001,
    };

    let perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &perm, &Permission::SetIRParams);
    let revoked_perm = Address::generate(&env);
    sut.pool
        .grant_permission(&sut.pool_admin, &revoked_perm, &Permission::SetIRParams);
    sut.pool
        .revoke_permission(&sut.pool_admin, &revoked_perm, &Permission::SetIRParams);
    let permissioned = sut.pool.permissioned(&Permission::SetIRParams);

    assert!(permissioned.binary_search(&revoked_perm).is_err());

    sut.pool.set_ir_params(&revoked_perm, &ir_params_input);
}
