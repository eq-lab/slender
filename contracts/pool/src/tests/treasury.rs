use crate::*;
use soroban_sdk::testutils::Address as _;

#[test]
fn should_return_treasury_address() {
    let env = Env::default();
    env.mock_all_auths();

    let pool = LendingPoolClient::new(&env, &env.register_contract(None, LendingPool));

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let flash_loan_fee = 5;
    let initial_health = 2_500;

    pool.initialize(
        &admin,
        &treasury,
        &flash_loan_fee,
        &initial_health,
        &IRParams {
            alpha: 143,
            initial_rate: 200,
            max_rate: 50_000,
            scaling_coeff: 9_000,
        },
    );

    assert_eq!(pool.treasury(), treasury);
}
