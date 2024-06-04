use crate::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn t() {
    let e = Env::default();
    e.mock_all_auths();
    let pool = LendingPoolClient::new(&e, &e.register_contract(None, LendingPool));
    let pool_admin = Address::generate(&e);

    pool.initialize(
        &pool_admin,
        &1,
        &25,
        &IRParams {
            alpha: 143,
            initial_rate: 200,
            max_rate: 50_000,
            scaling_coeff: 9_000,
        },
        &1,
    );

    let permissioned = pool.permissioned(&pool_admin, &Permission::Permisssion);

    extern crate std;

    std::println!("{:?}", permissioned);

    for p in [
        Permission::SetPriceFeeds,
        Permission::InitReserve,
        Permission::CollateralReserveParams,
    ] {
        pool.grant_permission(&pool_admin, &pool_admin, &p);
    }
}
