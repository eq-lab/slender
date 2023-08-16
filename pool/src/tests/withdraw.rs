use super::sut::fill_pool;
use crate::tests::sut::{fill_pool_two, init_pool, DAY};
use crate::*;
use soroban_sdk::symbol_short;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, Events, Ledger};
use soroban_sdk::{vec, IntoVal, Symbol};

#[test]
fn should_require_authorized_caller() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, _) = fill_pool(&env, &sut, true);
    let token_address = sut.token().address.clone();

    sut.pool
        .withdraw(&borrower, &token_address, &1_000_000, &borrower);

    assert_eq!(
        env.auths().pop().map(|f| f.1.function).unwrap(),
        AuthorizedFunction::Contract((
            sut.pool.address.clone(),
            symbol_short!("withdraw"),
            (
                borrower.clone(),
                token_address,
                1_000_000i128,
                borrower.clone()
            )
                .into_val(&env)
        )),
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn should_fail_when_pool_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, _) = fill_pool(&env, &sut, true);
    let token_address = sut.token().address.clone();

    sut.pool.set_pause(&true);
    sut.pool
        .withdraw(&borrower, &token_address, &1_000_000, &borrower);

    // assert_eq!(
    //     sut.pool
    //         .try_withdraw(&borrower, &token_address, &1_000_000, &borrower)
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
    let (_, borrower, _) = fill_pool(&env, &sut, true);
    let token_address = sut.token().address.clone();

    sut.pool.withdraw(&borrower, &token_address, &-1, &borrower);

    // assert_eq!(
    //     sut.pool
    //         .try_withdraw(&borrower, &token_address, &-1, &borrower)
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
    let (_, borrower, _) = fill_pool(&env, &sut, true);
    let token_address = sut.token().address.clone();

    sut.pool.set_reserve_status(&token_address, &false);
    sut.pool
        .withdraw(&borrower, &token_address, &1_000_000, &borrower);

    // assert_eq!(
    //     sut.pool
    //         .try_withdraw(&borrower, &token_address, &1_000_000, &borrower)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::NoActiveReserve
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_not_enough_stoken_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, _) = fill_pool(&env, &sut, true);
    let token_address = sut.token().address.clone();

    sut.pool
        .withdraw(&borrower, &token_address, &200_000_000, &borrower);

    // assert_eq!(
    //     sut.pool
    //         .try_withdraw(&borrower, &token_address, &1_000_000, &borrower)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::NotEnoughAvailableUserBalance
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_bad_position() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, _) = fill_pool(&env, &sut, true);
    let token_address = sut.token().address.clone();

    sut.pool
        .withdraw(&borrower, &token_address, &50_000_000, &borrower);

    // assert_eq!(
    //     sut.pool
    //         .try_withdraw(&borrower, &token_address, &1_000_000, &borrower)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::BadPosition
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn should_fail_when_unknown_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let unknown_asset = Address::random(&env);
    let sut = init_pool(&env);
    let (_, borrower, _) = fill_pool(&env, &sut, true);

    sut.pool
        .withdraw(&borrower, &unknown_asset, &1_000_000, &borrower);

    // assert_eq!(
    //     sut.pool
    //         .try_withdraw(&borrower, &unknown_asset, &1_000_000, &borrower)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::NoReserveExistForAsset
    // )
}

#[test]
fn should_change_user_config() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::random(&env);
    let sut = init_pool(&env);
    let token_address = sut.token().address.clone();

    sut.token_admin().mint(&user, &1_000_000_000);
    sut.pool.deposit(&user, &token_address, &1_000_000_000);

    let user_config_before = sut.pool.user_configuration(&user);

    sut.pool
        .withdraw(&user, &token_address, &1_000_000_000, &user);

    let user_config = sut.pool.user_configuration(&user);
    let reserve = sut.pool.get_reserve(&token_address).unwrap();

    assert_eq!(
        user_config_before.is_using_as_collateral(&env, reserve.get_id()),
        true
    );
    assert_eq!(
        user_config.is_using_as_collateral(&env, reserve.get_id()),
        false
    );
}

#[test]
fn should_partially_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let (lender, _, _, debt_config) = fill_pool_two(&env, &sut);
    let debt_token = &debt_config.token.address;

    env.ledger().with_mut(|li| li.timestamp = 60 * DAY);

    let s_token_supply_before = debt_config.s_token.total_supply();
    let lender_stoken_balance_before = debt_config.s_token.balance(&lender);
    let lender_underlying_balance_before = debt_config.token.balance(&lender);
    let s_token_underlying_supply_before = sut
        .pool
        .stoken_underlying_balance(&debt_config.s_token.address);

    sut.pool.withdraw(&lender, debt_token, &50_000_000, &lender);

    let lender_stoken_balance = debt_config.s_token.balance(&lender);
    let lender_underlying_balance = debt_config.token.balance(&lender);
    let s_token_supply = debt_config.s_token.total_supply();
    let s_token_underlying_supply = sut
        .pool
        .stoken_underlying_balance(&debt_config.s_token.address);

    assert_eq!(lender_stoken_balance_before, 100_000_000);
    assert_eq!(lender_underlying_balance_before, 900_000_000);
    assert_eq!(s_token_supply_before, 200_000_220);
    assert_eq!(s_token_underlying_supply_before, 160_000_000);

    assert_eq!(lender_stoken_balance, 50_028_949);
    assert_eq!(lender_underlying_balance, 950_000_000);
    assert_eq!(s_token_supply, 150_029_169);
    assert_eq!(s_token_underlying_supply, 110_000_000);
}

#[test]
fn should_fully_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let (lender, _, _, debt_config) = fill_pool_two(&env, &sut);
    let debt_token = &debt_config.token.address;

    env.ledger().with_mut(|li| li.timestamp = 60 * DAY);

    let s_token_supply_before = debt_config.s_token.total_supply();
    let lender_stoken_balance_before = debt_config.s_token.balance(&lender);
    let lender_underlying_balance_before = debt_config.token.balance(&lender);
    let s_token_underlying_supply_before = sut
        .pool
        .stoken_underlying_balance(&debt_config.s_token.address);

    sut.pool.withdraw(&lender, debt_token, &i128::MAX, &lender);

    let lender_stoken_balance = debt_config.s_token.balance(&lender);
    let lender_underlying_balance = debt_config.token.balance(&lender);
    let s_token_supply = debt_config.s_token.total_supply();
    let s_token_underlying_supply = sut
        .pool
        .stoken_underlying_balance(&debt_config.s_token.address);

    assert_eq!(lender_stoken_balance_before, 100_000_000);
    assert_eq!(lender_underlying_balance_before, 900_000_000);
    assert_eq!(s_token_supply_before, 200_000_220);
    assert_eq!(s_token_underlying_supply_before, 160_000_000);

    assert_eq!(lender_stoken_balance, 0);
    assert_eq!(lender_underlying_balance, 1_000_057_931);
    assert_eq!(s_token_supply, 100_000_220);
    assert_eq!(s_token_underlying_supply, 59_942_069);
}

#[test]
fn should_affect_coeffs() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let (lender, _, _, debt_config) = fill_pool_two(&env, &sut);
    let debt_token = &debt_config.token.address;

    env.ledger().with_mut(|li| li.timestamp = 2 * DAY);

    let collat_coeff_prev = sut.pool.collat_coeff(&debt_token);
    let debt_coeff_prev = sut.pool.debt_coeff(&debt_token);

    sut.pool.withdraw(&lender, debt_token, &i128::MAX, &lender);

    env.ledger().with_mut(|li| li.timestamp = 3 * DAY);

    let collat_coeff = sut.pool.collat_coeff(&debt_token);
    let debt_coeff = sut.pool.debt_coeff(&debt_token);

    assert!(collat_coeff_prev < collat_coeff);
    assert!(debt_coeff_prev < debt_coeff);
}

#[test]
fn should_affect_account_data() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, _) = fill_pool(&env, &sut, true);
    let token_address = sut.token().address.clone();

    let account_position_prev = sut.pool.account_position(&borrower);

    sut.pool
        .withdraw(&borrower, &token_address, &10_000_000, &borrower);

    let account_position = sut.pool.account_position(&borrower);

    assert!(account_position_prev.discounted_collateral > account_position.discounted_collateral);
    assert!(account_position_prev.debt == account_position.debt);
    assert!(account_position_prev.npv > account_position.npv);
}

#[test]
fn should_allow_withdraw_to_other_address() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let (lender, borrower, _, debt_config) = fill_pool_two(&env, &sut);
    let debt_token = &debt_config.token.address;

    env.ledger().with_mut(|li| li.timestamp = 60 * DAY);

    let borrower_underlying_balance_before = debt_config.token.balance(&borrower);
    let lender_stoken_balance_before = debt_config.s_token.balance(&lender);
    let lender_underlying_balance_before = debt_config.token.balance(&lender);
    let s_token_supply_before = debt_config.s_token.total_supply();
    let s_token_underlying_supply_before = sut
        .pool
        .stoken_underlying_balance(&debt_config.s_token.address);

    sut.pool
        .withdraw(&lender, debt_token, &50_000_000, &borrower);

    let borrower_underlying_balance = debt_config.token.balance(&borrower);
    let lender_stoken_balance = debt_config.s_token.balance(&lender);
    let lender_underlying_balance = debt_config.token.balance(&lender);
    let s_token_supply = debt_config.s_token.total_supply();
    let s_token_underlying_supply = sut
        .pool
        .stoken_underlying_balance(&debt_config.s_token.address);

    assert_eq!(borrower_underlying_balance_before, 900_000_000);
    assert_eq!(lender_stoken_balance_before, 100_000_000);
    assert_eq!(lender_underlying_balance_before, 900_000_000);
    assert_eq!(s_token_supply_before, 200_000_220);
    assert_eq!(s_token_underlying_supply_before, 160_000_000);

    assert_eq!(borrower_underlying_balance, 950000000);
    assert_eq!(lender_stoken_balance, 50_028_949);
    assert_eq!(lender_underlying_balance, 900_000_000);
    assert_eq!(s_token_supply, 150_029_169);
    assert_eq!(s_token_underlying_supply, 110_000_000);
}

#[test]
fn should_emit_events() {
    let env = Env::default();
    env.mock_all_auths();

    let user_1 = Address::random(&env);
    let user_2 = Address::random(&env);

    let sut = init_pool(&env);
    let token_address = sut.token().address.clone();

    sut.token_admin().mint(&user_1, &1_000_000_000);
    sut.pool.deposit(&user_1, &token_address, &1_000_000_000);

    sut.pool
        .withdraw(&user_1, &token_address, &1_000_000_000, &user_2);

    let mut events = env.events().all();
    let event = events.pop_back_unchecked();

    assert_eq!(
        vec![&env, event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "withdraw"), user_1.clone()).into_val(&env),
                (user_2.clone(), token_address.clone(), 1_000_000_000i128).into_val(&env)
            ),
        ]
    );

    let event = events.pop_back_unchecked();

    assert_eq!(
        vec![&env, event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (
                    Symbol::new(&env, "reserve_used_as_coll_disabled"),
                    user_1.clone()
                )
                    .into_val(&env),
                (token_address).into_val(&env)
            ),
        ]
    );
}
