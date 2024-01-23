use super::sut::DAY;
use crate::tests::sut::{fill_pool, init_pool};
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, Events, Ledger};
use soroban_sdk::{symbol_short, vec, Address, Env, IntoVal, Symbol};

#[test]
fn should_require_authorized_caller() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
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

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.pool.set_pause(&true);
    sut.pool.borrow(&borrower, &token_address, &10_000_000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #304)")]
fn should_fail_when_invalid_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.pool.borrow(&borrower, &token_address, &-1);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #101)")]
fn should_fail_when_reserve_deactivated() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.pool.set_reserve_status(&token_address, &false);
    sut.pool.borrow(&borrower, &token_address, &10_000_000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #300)")]
fn should_fail_when_borrowing_disabled() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.pool.enable_borrowing_on_reserve(&token_address, &false);
    sut.pool.borrow(&borrower, &token_address, &10_000_000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #309)")]
fn should_fail_when_borrowing_collat_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.pool.deposit(&borrower, &token_address, &10_000);
    sut.pool.borrow(&borrower, &token_address, &10_000_000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #310)")]
fn should_fail_when_util_cap_exceeded() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.pool
        .deposit(&borrower, &sut.reserves[0].token.address, &1_000_000);

    sut.pool.borrow(&borrower, &token_address, &100_000_000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #301)")]
fn should_fail_when_collat_not_covers_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.pool.borrow(&borrower, &token_address, &61_000_000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #202)")]
fn should_fail_when_user_config_not_exist() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let borrower = Address::generate(&env);

    sut.pool.borrow(&borrower, &sut.token().address, &1);
}

#[test]
fn should_change_user_config() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_1_address = debt_config.token.address.clone();
    let token_2_address = sut.reserves[2].token.address.clone();

    let reserve_1 = sut.pool.get_reserve(&token_1_address).unwrap();
    let reserve_2 = sut.pool.get_reserve(&token_2_address).unwrap();

    let user_config = sut.pool.user_configuration(&borrower);
    let is_borrowing_any_before = user_config.is_borrowing_any();
    let is_borrowing_token_1_before = user_config.is_borrowing(&env, reserve_1.get_id());
    let is_borrowing_token_2_before = user_config.is_borrowing(&env, reserve_2.get_id());

    sut.pool.borrow(&borrower, &token_1_address, &10_000_000);
    sut.pool.borrow(&borrower, &token_2_address, &10_000_000);

    let user_config = sut.pool.user_configuration(&borrower);
    let is_borrowing_any_after_borrow = user_config.is_borrowing_any();
    let is_borrowing_token_1_after_borrow = user_config.is_borrowing(&env, reserve_1.get_id());
    let is_borrowing_token_2_after_borrow = user_config.is_borrowing(&env, reserve_2.get_id());

    sut.pool.repay(&borrower, &token_1_address, &i128::MAX);

    let user_config = sut.pool.user_configuration(&borrower);
    let is_borrowing_any_after_repay_1 = user_config.is_borrowing_any();
    let is_borrowing_token_1_after_repay_1 = user_config.is_borrowing(&env, reserve_1.get_id());
    let is_borrowing_token_2_after_repay_1 = user_config.is_borrowing(&env, reserve_2.get_id());

    sut.pool.repay(&borrower, &token_2_address, &i128::MAX);

    let user_config = sut.pool.user_configuration(&borrower);
    let is_borrowing_any_after_repay_2 = user_config.is_borrowing_any();
    let is_borrowing_token_1_after_repay_2 = user_config.is_borrowing(&env, reserve_1.get_id());
    let is_borrowing_token_2_after_repay_2 = user_config.is_borrowing(&env, reserve_2.get_id());

    assert_eq!(is_borrowing_any_before, false);
    assert_eq!(is_borrowing_token_1_before, false);
    assert_eq!(is_borrowing_token_2_before, false);

    assert_eq!(is_borrowing_any_after_borrow, true);
    assert_eq!(is_borrowing_token_1_after_borrow, true);
    assert_eq!(is_borrowing_token_2_after_borrow, true);

    assert_eq!(is_borrowing_any_after_repay_1, true);
    assert_eq!(is_borrowing_token_1_after_repay_1, false);
    assert_eq!(is_borrowing_token_2_after_repay_1, true);

    assert_eq!(is_borrowing_any_after_repay_2, false);
    assert_eq!(is_borrowing_token_1_after_repay_2, false);
    assert_eq!(is_borrowing_token_2_after_repay_2, false);
}

#[test]
fn should_affect_coeffs() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    env.ledger().with_mut(|li| li.timestamp = 2 * DAY);

    let collat_coeff_prev = sut.pool.collat_coeff(&token_address);
    let debt_coeff_prev = sut.pool.debt_coeff(&token_address);

    sut.pool.borrow(&borrower, &token_address, &20_000_000);

    env.ledger().with_mut(|li| li.timestamp = 3 * DAY);

    let collat_coeff = sut.pool.collat_coeff(&token_address);
    let debt_coeff = sut.pool.debt_coeff(&token_address);

    assert!(collat_coeff_prev < collat_coeff);
    assert!(debt_coeff_prev < debt_coeff);
}

#[test]
fn should_affect_account_data() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    let account_position_prev = sut.pool.account_position(&borrower);

    sut.pool.borrow(&borrower, &token_address, &20_000_000);

    let account_position = sut.pool.account_position(&borrower);

    let debt_token_total_supply = debt_config.debt_token.total_supply();
    let pool_debt_token_total_supply = sut.pool.token_total_supply(&debt_config.debt_token.address);

    let debt_token_balance = debt_config.debt_token.balance(&borrower);
    let pool_debt_token_balance = sut
        .pool
        .token_balance(&debt_config.debt_token.address, &borrower);

    assert_eq!(debt_token_total_supply, pool_debt_token_total_supply);
    assert_eq!(debt_token_balance, pool_debt_token_balance);

    assert!(account_position_prev.discounted_collateral == account_position.discounted_collateral);
    assert!(account_position_prev.debt < account_position.debt);
    assert!(account_position_prev.npv > account_position.npv);
}

#[test]
fn should_change_balances_when_borrow_and_repay() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();
    let treasury = sut.pool.treasury();

    env.ledger().with_mut(|li| li.timestamp = 2 * DAY);

    let treasury_before = debt_config.token.balance(&treasury);
    let debt_balance_before = debt_config.debt_token.balance(&borrower);
    let debt_total_before = debt_config.debt_token.total_supply();
    let borrower_balance_before = debt_config.token.balance(&borrower);
    let underlying_supply_before = sut
        .pool
        .stoken_underlying_balance(&debt_config.s_token.address);

    sut.pool.borrow(&borrower, &token_address, &20_000_000);

    let treasury_after_borrow = debt_config.token.balance(&treasury);
    let debt_balance_after_borrow = debt_config.debt_token.balance(&borrower);
    let debt_total_after_borrow = debt_config.debt_token.total_supply();
    let borrower_balance_after_borrow = debt_config.token.balance(&borrower);
    let underlying_supply_after_borrow = sut
        .pool
        .stoken_underlying_balance(&debt_config.s_token.address);

    env.ledger().with_mut(|li| li.timestamp = 30 * DAY);

    sut.pool.repay(&borrower, &token_address, &i128::MAX);

    let treasury_after_repay = debt_config.token.balance(&treasury);
    let debt_balance_after_repay = debt_config.debt_token.balance(&borrower);
    let debt_total_after_repay = debt_config.debt_token.total_supply();
    let borrower_balance_after_repay = debt_config.token.balance(&borrower);
    let underlying_supply_after_repay = sut
        .pool
        .stoken_underlying_balance(&debt_config.s_token.address);

    assert_eq!(treasury_before, 0);
    assert_eq!(debt_balance_before, 0);
    assert_eq!(debt_total_before, 0);
    assert_eq!(borrower_balance_before, 1_000_000_000);
    assert_eq!(underlying_supply_before, 100_000_000);

    assert_eq!(treasury_after_borrow, 0);
    assert_eq!(debt_balance_after_borrow, 20_000_001);
    assert_eq!(debt_total_after_borrow, 20_000_001);
    assert_eq!(borrower_balance_after_borrow, 1_020_000_000);
    assert_eq!(underlying_supply_after_borrow, 80_000_000);

    assert_eq!(treasury_after_repay, 37_156);
    assert_eq!(debt_balance_after_repay, 0);
    assert_eq!(debt_total_after_repay, 0);
    assert_eq!(borrower_balance_after_repay, 999_954_789);
    assert_eq!(underlying_supply_after_repay, 100_008_055);
}

#[test]
fn should_emit_events() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    sut.pool.borrow(&borrower, &token_address, &20_000_000);

    let mut events = env.events().all();
    let event = events.pop_back_unchecked();

    assert_eq!(
        vec![&env, event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "borrow"), borrower.clone()).into_val(&env),
                (token_address.clone(), 20_000_000i128).into_val(&env)
            ),
        ]
    );
}

// TODO: remove
// #[test]
// fn should_emit_events() {
//     let env = Env::default();
//     env.mock_all_auths();

//     let sut = init_pool(&env, false);
//     // TODO: uncomment
//     // let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
//     // let token_address = debt_config.token.address.clone();

//     // sut.pool.borrow(&borrower, &token_address, &20_000_000);

//     // TODO: remove
//     let lender = Address::generate(&env);
//     let borrower = Address::generate(&env);

//     for i in 0..3 {
//         let amount = 100_000_000_000_000;

//         sut.reserves[i].token_admin.mint(&lender, &amount);
//         sut.reserves[i].token_admin.mint(&borrower, &amount);

//         assert_eq!(sut.reserves[i].token.balance(&lender), amount);
//         assert_eq!(sut.reserves[i].token.balance(&borrower), amount);
//     }

//     //lender deposit all tokens
//     for i in 0..3 {
//         let amount = 10_000_000_000_000;
//         let stoken = sut.reserves[i].s_token.address.clone();
//         let token = sut.reserves[i].token.address.clone();
//         let pool_balance = sut.reserves[i].token.balance(&stoken);

//         sut.pool.deposit(&lender, &token, &amount);

//         assert_eq!(sut.reserves[i].s_token.balance(&lender), amount);
//         assert_eq!(
//             sut.reserves[i].token.balance(&stoken),
//             pool_balance + amount
//         );
//     }

//     let debt_feed_address = sut
//         .pool
//         .price_feeds(&sut.reserves[2].token.address.clone())
//         .unwrap()
//         .feeds
//         .get_unchecked(0)
//         .feed;
//     let feed = PriceFeedClient::new(&env, &debt_feed_address);
//     feed.init(
//         &Asset::Stellar(sut.reserves[2].token.address.clone()),
//         &vec![
//             &env,
//             PriceData {
//                 price: 1_000_000_000_000_000,
//                 timestamp: 1704790200000,
//             },
//         ],
//     );
//     // env.ledger().with_mut(|li| li.timestamp = DAY);

//     //borrower deposit first token and borrow second token
//     sut.pool
//         .deposit(&borrower, &sut.reserves[0].token.address, &2_000_000_000);
//     sut.pool.deposit(
//         &borrower,
//         &sut.reserves[1].token.address,
//         &1_000_000_000_000,
//     );

//     // sut.pool.borrow(
//     //     &borrower,
//     //     &sut.reserves[1].token.address.clone(),
//     //     &400_000_000_000,
//     // );
//     sut.pool.borrow(
//         &borrower,
//         &sut.reserves[2].token.address.clone(),
//         &800_000_000_000,
//     );

//     feed.init(
//         &Asset::Stellar(sut.reserves[2].token.address.clone()),
//         &vec![
//             &env,
//             PriceData {
//                 price: 11_000_000_000_000_000,
//                 timestamp: 1704790200000,
//             },
//         ],
//     );

//     let _account_position = sut.pool.account_position(&borrower);

//     sut.pool.liquidate(&lender, &borrower, &false);

//     let _account_position = sut.pool.account_position(&borrower);
//     let _account_position = sut.pool.account_position(&borrower);

//     // let mut events = env.events().all();
//     // let event = events.pop_back_unchecked();

//     // assert_eq!(
//     //     vec![&env, event],
//     //     vec![
//     //         &env,
//     //         (
//     //             sut.pool.address.clone(),
//     //             (Symbol::new(&env, "borrow"), borrower.clone()).into_val(&env),
//     //             (token_address.clone(), 20_000_000i128).into_val(&env)
//     //         ),
//     //     ]
//     // );
// }
