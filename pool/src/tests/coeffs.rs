use crate::tests::sut::{init_pool, DAY};
use crate::*;
use soroban_sdk::testutils::{Address as _, Ledger};

// ARs being updated over time /Artur

#[test]
fn should_update_coeffs_when_deposit_borrow_withdraw_liquidate() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let debt_asset = sut.reserves[1].token.address.clone();

    let lender = Address::random(&env);
    let borrower = Address::random(&env);

    for r in sut.reserves.iter() {
        r.token_admin.mint(&lender, &1_000_000_000);
        r.token_admin.mint(&borrower, &1_000_000_000);
    }

    let _collat_coeff_1 = sut.pool.collat_coeff(&debt_asset);
    let _debt_coeff_1 = sut.pool.debt_coeff(&debt_asset);

    env.ledger().with_mut(|l| l.timestamp = DAY);

    for r in sut.reserves.iter() {
        sut.pool.deposit(&lender, &r.token.address, &100_000_000);
    }

    env.ledger().with_mut(|l| l.timestamp = 2 * DAY);

    let _collat_coeff_1 = sut.pool.collat_coeff(&debt_asset);
    let _debt_coeff_1 = sut.pool.debt_coeff(&debt_asset);
    let _debt_coeff_1 = sut.pool.collat_coeff(&debt_asset);
}

// #[test]
// fn collateral_coeff_test() {
//     let env = Env::default();
//     env.mock_all_auths();

//     let sut = init_pool(&env);

//     let (_, borrower, debt_config) = fill_pool(&env, &sut, true);
//     let initial_collat_coeff = sut.pool.collat_coeff(&debt_config.token.address);
//     std::println!("initial_collat_coeff={}", initial_collat_coeff);

//     env.ledger().with_mut(|l| l.timestamp = 2 * DAY);

//     sut.pool
//         .borrow(&borrower, &debt_config.token.address, &50_000);
//     let reserve = sut.pool.get_reserve(&debt_config.token.address).unwrap();

//     let collat_ar = FixedI128::from_inner(reserve.lender_ar);
//     let s_token_supply = debt_config.s_token.total_supply();
//     let balance = debt_config.token.balance(&debt_config.s_token.address);
//     let debt_token_suply = debt_config.debt_token.total_supply();

//     let expected_collat_coeff = FixedI128::from_rational(
//         balance + collat_ar.mul_int(debt_token_suply).unwrap(),
//         s_token_supply,
//     )
//     .unwrap()
//     .into_inner();

//     let collat_coeff = sut.pool.collat_coeff(&debt_config.token.address);
//     assert_eq!(collat_coeff, expected_collat_coeff);

//     // shift time to 8 days
//     env.ledger().with_mut(|l| l.timestamp = 10 * DAY);

//     let elapsed_time = 8 * DAY;
//     let collat_ar = calc_next_accrued_rate(
//         collat_ar,
//         FixedI128::from_inner(reserve.lender_ir),
//         elapsed_time,
//     )
//     .unwrap();
//     let expected_collat_coeff = FixedI128::from_rational(
//         balance + collat_ar.mul_int(debt_token_suply).unwrap(),
//         s_token_supply,
//     )
//     .unwrap()
//     .into_inner();

//     let collat_coeff = sut.pool.collat_coeff(&debt_config.token.address);
//     assert_eq!(collat_coeff, expected_collat_coeff);
//     std::println!("collat_coeff={}", collat_coeff);
// }
