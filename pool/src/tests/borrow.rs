use soroban_sdk::testutils::AuthorizedFunction;
use soroban_sdk::{symbol_short, Env, IntoVal};

use crate::tests::sut::{fill_pool, init_pool};

// enable_borrowing_on_reserve /Artur

#[test]
fn should_require_authorized_caller() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.pool.borrow(&borrower, &token_address, &10_000_000);

    assert_eq!(
        env.auths().pop().map(|f| f.1.function).unwrap(),
        AuthorizedFunction::Contract((
            sut.pool.address.clone(),
            symbol_short!("borrow"),
            (borrower.clone(), token_address, 10_000_000i128,).into_val(&env)
        )),
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn should_fail_when_pool_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.pool.set_pause(&true);
    sut.pool.borrow(&borrower, &token_address, &10_000_000);

    // assert_eq!(
    //     sut.pool
    //         .try_borrow(&borrower, &token_address, &10_000_000)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::Paused
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_invalid_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.pool.borrow(&borrower, &token_address, &-1);

    // assert_eq!(
    //     sut.pool
    //         .try_borrow(&borrower, &token_address, &-1)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::InvalidAmount
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_reserve_deactivated() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.pool.set_reserve_status(&token_address, &false);
    sut.pool.borrow(&borrower, &token_address, &10_000_000);

    // assert_eq!(
    //     sut.pool
    //         .try_borrow(&borrower, &token_address, &10_000_000)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::NoActiveReserve
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_borrowing_disabled() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.pool.enable_borrowing_on_reserve(&token_address, &false);
    sut.pool.borrow(&borrower, &token_address, &10_000_000);

    // assert_eq!(
    //     sut.pool
    //         .try_borrow(&borrower, &token_address, &10_000_000)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::BorrowingNotEnabled
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_borrowing_collat_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.pool.deposit(&borrower, &token_address, &10_000);
    sut.pool.borrow(&borrower, &token_address, &10_000_000);

    // assert_eq!(
    //     sut.pool
    //         .try_borrow(&borrower, &token_address, &10_000_000)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::MustNotBeInCollateralAsset
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_util_cap_exceeded() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.pool.borrow(&borrower, &token_address, &100_000_000);

    // assert_eq!(
    //     sut.pool
    //         .try_borrow(&borrower, &token_address, &100_000_000)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::UtilizationCapExceeded
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_oracle_price_is_negative() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.price_feed.set_price(&token_address, &-1_000);
    sut.pool.borrow(&borrower, &token_address, &10_000_000);

    // assert_eq!(
    //     sut.pool
    //         .try_borrow(&borrower, &token_address, &10_000_000)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::ValidateBorrowMathError
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_collat_not_covers_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.pool.borrow(&borrower, &token_address, &61_000_000);

    // assert_eq!(
    //     sut.pool
    //         .try_borrow(&borrower, &token_address, &100_000_000)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::CollateralNotCoverNewBorrow
    // )
}

// #[test]
// #[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
// fn should_fail_when_unknown_asset() {
//     let env = Env::default();
//     env.mock_all_auths();

//     let unknown_asset = Address::random(&env);
//     let sut = init_pool(&env);
//     let (_, borrower, _) = fill_pool(&env, &sut, false);

//     sut.pool
//         .withdraw(&borrower, &unknown_asset, &1_000_000, &borrower);

//     // assert_eq!(
//     //     sut.pool
//     //         .try_withdraw(&borrower, &unknown_asset, &1_000_000, &borrower)
//     //         .unwrap_err()
//     //         .unwrap(),
//     //     Error::NoReserveExistForAsset
//     // )
// }

// #[test]
// fn borrow() {
//     let env = Env::default();
//     env.mock_all_auths();

//     let sut = init_pool(&env);

//     let initial_amount: i128 = 1_000_000_000;
//     let lender = Address::random(&env);
//     let borrower = Address::random(&env);

//     for r in sut.reserves.iter() {
//         r.token_admin.mint(&lender, &initial_amount);
//         assert_eq!(r.token.balance(&lender), initial_amount);

//         r.token_admin.mint(&borrower, &initial_amount);
//         assert_eq!(r.token.balance(&borrower), initial_amount);
//     }

//     //TODO: optimize gas
//     env.budget().reset_unlimited();

//     //lender deposit all tokens
//     let deposit_amount = 100_000_000;
//     for r in sut.reserves.iter() {
//         let pool_balance = r.token.balance(&r.s_token.address);
//         sut.pool.deposit(&lender, &r.token.address, &deposit_amount);
//         assert_eq!(r.s_token.balance(&lender), deposit_amount);
//         assert_eq!(
//             r.token.balance(&r.s_token.address),
//             pool_balance + deposit_amount
//         );
//     }

//     //borrower deposit first token and borrow second token
//     sut.pool
//         .deposit(&borrower, &sut.reserves[0].token.address, &deposit_amount);
//     assert_eq!(sut.reserves[0].s_token.balance(&borrower), deposit_amount);

//     let s_token_underlying_supply = sut
//         .pool
//         .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

//     assert_eq!(s_token_underlying_supply, 100_000_000);

//     //borrower borrow second token
//     let borrow_asset = sut.reserves[1].token.address.clone();
//     let borrow_amount = 10_000;
//     let pool_balance_before = sut.reserves[1]
//         .token
//         .balance(&sut.reserves[1].s_token.address);

//     let borrower_balance_before = sut.reserves[1].token.balance(&borrower);
//     sut.pool.borrow(&borrower, &borrow_asset, &borrow_amount);
//     assert_eq!(
//         sut.reserves[1].token.balance(&borrower),
//         borrower_balance_before + borrow_amount
//     );

//     let s_token_underlying_supply = sut
//         .pool
//         .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

//     let pool_balance = sut.reserves[1]
//         .token
//         .balance(&sut.reserves[1].s_token.address);
//     let debt_token_balance = sut.reserves[1].debt_token.balance(&borrower);
//     assert_eq!(
//         pool_balance + borrow_amount,
//         pool_balance_before,
//         "Pool balance"
//     );
//     assert_eq!(debt_token_balance, borrow_amount, "Debt token balance");
//     assert_eq!(s_token_underlying_supply, 99_990_000);
// }

// #[test]
// #[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
// fn borrow_utilization_exceeded() {
//     let env = Env::default();
//     env.mock_all_auths();

//     let sut = init_pool(&env);

//     let initial_amount: i128 = 1_000_000_000;
//     let lender = Address::random(&env);
//     let borrower = Address::random(&env);

//     sut.reserves[0].token_admin.mint(&lender, &initial_amount);
//     sut.reserves[1].token_admin.mint(&borrower, &initial_amount);

//     //TODO: optimize gas
//     env.budget().reset_unlimited();

//     let deposit_amount = 1_000_000_000;

//     sut.pool
//         .deposit(&lender, &sut.reserves[0].token.address, &deposit_amount);

//     sut.pool
//         .deposit(&borrower, &sut.reserves[1].token.address, &deposit_amount);

//     sut.pool
//         .borrow(&borrower, &sut.reserves[0].token.address, &990_000_000);

//     // assert_eq!(
//     //     sut.pool
//     //         .try_borrow(&borrower, &sut.reserves[0].token.address, &990_000_000)
//     //         .unwrap_err()
//     //         .unwrap(),
//     //     Error::UtilizationCapExceeded
//     // )
// }

// #[test]
// #[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
// fn borrow_user_confgig_not_exists() {
//     let env = Env::default();
//     env.mock_all_auths();

//     let sut = init_pool(&env);
//     let borrower = Address::random(&env);

//     //TODO: check error after soroban fix
//     let borrow_amount = 0;
//     sut.pool
//         .borrow(&borrower, &sut.reserves[0].token.address, &borrow_amount);
//     // assert_eq!(
//     //     sut.pool
//     //         .try_borrow(&borrower, &sut.reserves[0].token.address, &borrow_amount)
//     //         .unwrap_err()
//     //         .unwrap(),
//     //     Error::UserConfigNotExists
//     // )
// }

// #[test]
// #[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
// fn borrow_collateral_is_zero() {
//     let env = Env::default();
//     env.mock_all_auths();

//     let sut = init_pool(&env);
//     let lender = Address::random(&env);
//     let borrower = Address::random(&env);

//     let initial_amount = 1_000_000_000;
//     for r in sut.reserves.iter() {
//         r.token_admin.mint(&borrower, &initial_amount);
//         assert_eq!(r.token.balance(&borrower), initial_amount);
//         r.token_admin.mint(&lender, &initial_amount);
//         assert_eq!(r.token.balance(&lender), initial_amount);
//     }

//     let deposit_amount = 1000;

//     env.budget().reset_unlimited();

//     sut.pool
//         .deposit(&lender, &sut.reserves[0].token.address, &deposit_amount);

//     sut.pool
//         .deposit(&borrower, &sut.reserves[1].token.address, &deposit_amount);

//     sut.pool.withdraw(
//         &borrower,
//         &sut.reserves[1].token.address,
//         &deposit_amount,
//         &borrower,
//     );

//     let borrow_amount = 100;
//     sut.pool
//         .borrow(&borrower, &sut.reserves[0].token.address, &borrow_amount)

//     //TODO: check error after fix
//     // assert_eq!(
//     //     sut.pool
//     //         .try_borrow(&borrower, &sut.reserves[0].token.address, &borrow_amount)
//     //         .unwrap_err()
//     //         .unwrap(),
//     //     Error::CollateralNotCoverNewBorrow
//     // )
// }

// #[test]
// fn borrow_no_active_reserve() {
//     //TODO: implement
// }

// #[test]
// #[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
// fn borrow_collateral_not_cover_new_debt() {
//     let env = Env::default();
//     env.mock_all_auths();

//     let sut = init_pool(&env);
//     let lender = Address::random(&env);
//     let borrower = Address::random(&env);

//     let initial_amount = 1_000_000_000;
//     for r in sut.reserves.iter() {
//         r.token_admin.mint(&borrower, &initial_amount);
//         assert_eq!(r.token.balance(&borrower), initial_amount);
//         r.token_admin.mint(&lender, &initial_amount);
//         assert_eq!(r.token.balance(&lender), initial_amount);
//     }

//     let borrower_deposit_amount = 500;
//     let lender_deposit_amount = 2000;

//     //TODO: optimize gas
//     env.budget().reset_unlimited();

//     sut.pool.deposit(
//         &lender,
//         &sut.reserves[0].token.address,
//         &lender_deposit_amount,
//     );

//     sut.pool.deposit(
//         &borrower,
//         &sut.reserves[1].token.address,
//         &borrower_deposit_amount,
//     );

//     //TODO: check error after soroban fix
//     let borrow_amount = 1000;
//     sut.pool
//         .borrow(&borrower, &sut.reserves[0].token.address, &borrow_amount);

//     // assert_eq!(
//     //     sut.pool
//     //         .try_borrow(&borrower, &sut.reserves[0].token.address, &borrow_amount)
//     //         .unwrap_err()
//     //         .unwrap(),
//     //     Error::CollateralNotCoverNewBorrow
//     // )
// }

// #[test]
// #[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
// fn borrow_disabled_for_borrowing_asset() {
//     let env = Env::default();
//     env.mock_all_auths();

//     let sut = init_pool(&env);

//     let initial_amount: i128 = 1_000_000_000;
//     let lender = Address::random(&env);
//     let borrower = Address::random(&env);

//     for r in sut.reserves.iter() {
//         r.token_admin.mint(&lender, &initial_amount);
//         assert_eq!(r.token.balance(&lender), initial_amount);

//         r.token_admin.mint(&borrower, &initial_amount);
//         assert_eq!(r.token.balance(&borrower), initial_amount);
//     }

//     env.budget().reset_unlimited();

//     //lender deposit all tokens
//     let deposit_amount = 100_000_000;
//     for r in sut.reserves.iter() {
//         let pool_balance = r.token.balance(&r.s_token.address);
//         sut.pool.deposit(&lender, &r.token.address, &deposit_amount);
//         assert_eq!(r.s_token.balance(&lender), deposit_amount);
//         assert_eq!(
//             r.token.balance(&r.s_token.address),
//             pool_balance + deposit_amount
//         );
//     }

//     //borrower deposit first token and borrow second token
//     sut.pool
//         .deposit(&borrower, &sut.reserves[0].token.address, &deposit_amount);
//     assert_eq!(sut.reserves[0].s_token.balance(&borrower), deposit_amount);

//     //borrower borrow second token
//     let borrow_asset = sut.reserves[1].token.address.clone();
//     let borrow_amount = 10_000;

//     //disable second token for borrowing
//     sut.pool.enable_borrowing_on_reserve(&borrow_asset, &false);
//     let reserve = sut.pool.get_reserve(&borrow_asset);
//     assert_eq!(reserve.unwrap().configuration.borrowing_enabled, false);

//     //TODO: check error after soroban fix
//     sut.pool.borrow(&borrower, &borrow_asset, &borrow_amount);

//     // assert_eq!(
//     //     sut.pool
//     //         .try_borrow(&borrower, &borrow_asset, &borrow_amount)
//     //         .unwrap_err()
//     //         .unwrap(),
//     //     Error::BorrowingNotEnabled
//     // );
// }

// #[test]
// fn borrow_should_mint_debt_token() {
//     let env = Env::default();
//     env.mock_all_auths();

//     //TODO: optimize gas

//     let sut = init_pool(&env);

//     env.budget().reset_unlimited();

//     let (_lender, borrower, debt_config) = fill_pool(&env, &sut, false);
//     let debt_token = &debt_config.token.address;

//     // shift time to one day
//     env.ledger().with_mut(|li| {
//         li.timestamp = 24 * 60 * 60 // one day
//     });

//     let debttoken_supply = debt_config.debt_token.total_supply();
//     let borrower_debt_token_balance_before = debt_config.debt_token.balance(&borrower);
//     let borrow_amount = 10_000;
//     sut.pool.borrow(&borrower, &debt_token, &borrow_amount);

//     let reserve = sut.pool.get_reserve(&debt_token).unwrap();
//     let expected_minted_debt_token = FixedI128::from_inner(reserve.borrower_accrued_rate)
//         .recip_mul_int(borrow_amount)
//         .unwrap();

//     assert_eq!(
//         debt_config.debt_token.balance(&borrower),
//         borrower_debt_token_balance_before + expected_minted_debt_token
//     );
//     assert_eq!(
//         debt_config.debt_token.balance(&borrower),
//         debttoken_supply + expected_minted_debt_token
//     )
// }
