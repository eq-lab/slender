use super::sut::fill_pool;
use crate::tests::sut::{fill_pool_two, init_pool, DAY};
use crate::*;
use soroban_sdk::symbol_short;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, Events};
use soroban_sdk::{vec, IntoVal, Symbol};
use tests::sut::set_time;

#[test]
fn should_require_authorized_caller() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, true);
    let token_address = sut.token().address.clone();

    set_time(&env, &sut, 2 * DAY, false);

    sut.pool
        .withdraw(&borrower, &token_address, &10_000, &borrower);

    assert_eq!(
        env.auths().pop().map(|f| f.1.function).unwrap(),
        AuthorizedFunction::Contract((
            sut.pool.address.clone(),
            symbol_short!("withdraw"),
            (
                borrower.clone(),
                token_address,
                10_000i128,
                borrower.clone()
            )
                .into_val(&env)
        )),
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn should_fail_when_pool_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, true);
    let token_address = sut.token().address.clone();

    set_time(&env, &sut, 2 * DAY, false);

    sut.pool.set_pause(&true);
    sut.pool
        .withdraw(&borrower, &token_address, &1_000_000, &borrower);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #302)")]
fn should_fail_when_invalid_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, true);
    let token_address = sut.token().address.clone();

    set_time(&env, &sut, 2 * DAY, false);

    sut.pool.withdraw(&borrower, &token_address, &-1, &borrower);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #100)")]
fn should_fail_when_reserve_deactivated() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, true);
    let token_address = sut.token().address.clone();

    set_time(&env, &sut, 2 * DAY, false);

    sut.pool.set_reserve_status(&token_address, &false);
    sut.pool
        .withdraw(&borrower, &token_address, &1_000_000, &borrower);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn should_fail_when_bellow_initial_health() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, true);
    let token_address = sut.token().address.clone();

    set_time(&env, &sut, 2 * DAY, false);

    sut.pool
        .withdraw(&borrower, &token_address, &50_000_000, &borrower);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn should_fail_when_unknown_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let unknown_asset = Address::generate(&env);
    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, true);

    set_time(&env, &sut, 2 * DAY, false);

    sut.pool
        .withdraw(&borrower, &unknown_asset, &1_000_000, &borrower);
}

#[test]
fn should_change_user_config() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);
    let sut = init_pool(&env, false);
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
    assert_eq!(user_config_before.total_assets(), 1);
    assert_eq!(
        user_config.is_using_as_collateral(&env, reserve.get_id()),
        false
    );
    assert_eq!(user_config.total_assets(), 0);
}

#[test]
fn should_partially_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let (lender, _, _, debt_config) = fill_pool_two(&env, &sut);
    let debt_token = &debt_config.token.address;

    set_time(&env, &sut, 60 * DAY, false);

    let s_token_supply_before = debt_config.s_token().total_supply();
    let lender_stoken_balance_before = debt_config.s_token().balance(&lender);
    let lender_underlying_balance_before = debt_config.token.balance(&lender);
    let s_token_underlying_supply_before = sut
        .pool
        .token_balance(&debt_config.token.address, &debt_config.s_token().address);

    sut.pool.withdraw(&lender, debt_token, &50_000_000, &lender);

    let lender_stoken_balance = debt_config.s_token().balance(&lender);
    let lender_underlying_balance = debt_config.token.balance(&lender);
    let s_token_supply = debt_config.s_token().total_supply();
    let s_token_underlying_supply = sut
        .pool
        .token_balance(&debt_config.token.address, &debt_config.s_token().address);

    assert_eq!(lender_stoken_balance_before, 100_000_000);
    assert_eq!(lender_underlying_balance_before, 900_000_000);
    assert_eq!(s_token_supply_before, 199_991_812);
    assert_eq!(s_token_underlying_supply_before, 160_000_000);

    assert_eq!(lender_stoken_balance, 50_043_047);
    assert_eq!(lender_underlying_balance, 950_000_000);
    assert_eq!(s_token_supply, 150_034_859);
    assert_eq!(s_token_underlying_supply, 110_000_000);
}

#[test]
fn should_fully_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let (lender, _, _, debt_config) = fill_pool_two(&env, &sut);
    let debt_token = &debt_config.token.address;

    set_time(&env, &sut, 60 * DAY, false);

    let s_token_supply_before = debt_config.s_token().total_supply();
    let lender_stoken_balance_before = debt_config.s_token().balance(&lender);
    let lender_underlying_balance_before = debt_config.token.balance(&lender);
    let s_token_underlying_supply_before = sut
        .pool
        .token_balance(&debt_config.token.address, &debt_config.s_token().address);

    sut.pool.withdraw(&lender, debt_token, &i128::MAX, &lender);

    set_time(&env, &sut, 60 * DAY + 1, false);

    let lender_stoken_balance = debt_config.s_token().balance(&lender);
    let lender_underlying_balance = debt_config.token.balance(&lender);
    let s_token_supply = debt_config.s_token().total_supply();
    let s_token_underlying_supply = sut
        .pool
        .token_balance(&debt_config.token.address, &debt_config.s_token().address);

    assert_eq!(lender_stoken_balance_before, 100_000_000);
    assert_eq!(lender_underlying_balance_before, 900_000_000);
    assert_eq!(s_token_supply_before, 199_991_812);
    assert_eq!(s_token_underlying_supply_before, 160_000_000);

    assert_eq!(lender_stoken_balance, 0);
    assert_eq!(lender_underlying_balance, 1_000_086_169);
    assert_eq!(s_token_supply, 99_991_812);
    assert_eq!(s_token_underlying_supply, 59_913_831);
}

#[test]
fn should_affect_coeffs() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let (lender, _, _, debt_config) = fill_pool_two(&env, &sut);
    let debt_token = &debt_config.token.address;

    set_time(&env, &sut, 2 * DAY, false);

    let collat_coeff_prev = sut.pool.collat_coeff(&debt_token);
    let debt_coeff_prev = sut.pool.debt_coeff(&debt_token);

    sut.pool.withdraw(&lender, debt_token, &i128::MAX, &lender);

    set_time(&env, &sut, 3 * DAY, false);

    let collat_coeff = sut.pool.collat_coeff(&debt_token);
    let debt_coeff = sut.pool.debt_coeff(&debt_token);

    assert!(collat_coeff_prev < collat_coeff);
    assert!(debt_coeff_prev < debt_coeff);
}

#[test]
fn should_affect_account_data() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, true);
    let token_address = sut.token().address.clone();

    let account_position_prev = sut.pool.account_position(&borrower);

    set_time(&env, &sut, 2 * DAY, false);

    sut.pool
        .withdraw(&borrower, &token_address, &100_000, &borrower);

    let account_position = sut.pool.account_position(&borrower);

    set_time(&env, &sut, 2 * DAY + 1, false);

    let collat_token_total_supply = sut.s_token().total_supply();
    let pool_collat_token_total_supply = sut.pool.token_total_supply(&sut.s_token().address);

    let collat_token_balance = sut.s_token().balance(&borrower);
    let pool_collat_token_balance = sut.pool.token_balance(&sut.s_token().address, &borrower);

    assert_eq!(collat_token_total_supply, pool_collat_token_total_supply);
    assert_eq!(collat_token_balance, pool_collat_token_balance);

    assert!(account_position_prev.discounted_collateral > account_position.discounted_collateral);
    assert!(account_position_prev.debt < account_position.debt);
    assert!(account_position_prev.npv > account_position.npv);
}

#[test]
fn should_allow_withdraw_to_other_address() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let (lender, borrower, _, debt_config) = fill_pool_two(&env, &sut);
    let debt_token = &debt_config.token.address;

    set_time(&env, &sut, 60 * DAY, false);

    let borrower_underlying_balance_before = debt_config.token.balance(&borrower);
    let lender_stoken_balance_before = debt_config.s_token().balance(&lender);
    let lender_underlying_balance_before = debt_config.token.balance(&lender);
    let s_token_supply_before = debt_config.s_token().total_supply();
    let s_token_underlying_supply_before = sut
        .pool
        .token_balance(&debt_config.token.address, &debt_config.s_token().address);

    sut.pool
        .withdraw(&lender, debt_token, &50_000_000, &borrower);

    let borrower_underlying_balance = debt_config.token.balance(&borrower);
    let lender_stoken_balance = debt_config.s_token().balance(&lender);
    let lender_underlying_balance = debt_config.token.balance(&lender);
    let s_token_supply = debt_config.s_token().total_supply();
    let s_token_underlying_supply = sut
        .pool
        .token_balance(&debt_config.token.address, &debt_config.s_token().address);

    assert_eq!(borrower_underlying_balance_before, 900_000_000);
    assert_eq!(lender_stoken_balance_before, 100_000_000);
    assert_eq!(lender_underlying_balance_before, 900_000_000);
    assert_eq!(s_token_supply_before, 199_991_812);
    assert_eq!(s_token_underlying_supply_before, 160_000_000);

    assert_eq!(borrower_underlying_balance, 950000000);
    assert_eq!(lender_stoken_balance, 50_043_047);
    assert_eq!(lender_underlying_balance, 900_000_000);
    assert_eq!(s_token_supply, 150_034_859);
    assert_eq!(s_token_underlying_supply, 110_000_000);
}

#[test]
fn should_emit_events() {
    let env = Env::default();
    env.mock_all_auths();

    let user_1 = Address::generate(&env);
    let user_2 = Address::generate(&env);

    let sut = init_pool(&env, false);
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

    let event = events.get(23).unwrap();

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

#[test]
fn rwa_partially_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let (lender, _, _, _) = fill_pool_two(&env, &sut);
    let rwa_config = sut.rwa_config();
    rwa_config.token_admin.mint(&lender, &100_000_000);
    sut.pool
        .deposit(&lender, &rwa_config.token.address, &100_000_000);

    set_time(&env, &sut, 60 * DAY, false);

    let lender_rwa_balance_before = rwa_config.token.balance(&lender);
    let pool_rwa_balance_before = rwa_config.token.balance(&sut.pool.address);

    sut.pool
        .withdraw(&lender, &rwa_config.token.address, &50_000_000, &lender);

    set_time(&env, &sut, 60 * DAY + 1, false);

    let lender_rwa_balance = rwa_config.token.balance(&lender);
    let pool_rwa_balance = rwa_config.token.balance(&sut.pool.address);

    assert_eq!(lender_rwa_balance_before, 0);
    assert_eq!(lender_rwa_balance, 50_000_000);
    assert_eq!(pool_rwa_balance_before, 100_000_000);
    assert_eq!(pool_rwa_balance, 50_000_000);
}

#[test]
fn rwa_fully_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let (lender, _, _, _) = fill_pool_two(&env, &sut);
    let rwa_config = sut.rwa_config();
    rwa_config.token_admin.mint(&lender, &100_000_000);
    sut.pool
        .deposit(&lender, &rwa_config.token.address, &100_000_000);

    set_time(&env, &sut, 60 * DAY, false);

    let lender_rwa_balance_before = rwa_config.token.balance(&lender);
    let pool_rwa_balance_before = rwa_config.token.balance(&sut.pool.address);

    sut.pool
        .withdraw(&lender, &rwa_config.token.address, &i128::MAX, &lender);

    set_time(&env, &sut, 60 * DAY + 1, false);

    let lender_rwa_balance = rwa_config.token.balance(&lender);
    let pool_rwa_balance = rwa_config.token.balance(&sut.pool.address);

    assert_eq!(lender_rwa_balance_before, 0);
    assert_eq!(lender_rwa_balance, 100_000_000);
    assert_eq!(pool_rwa_balance_before, 100_000_000);
    assert_eq!(pool_rwa_balance, 0);
}

#[test]
fn rwa_should_not_affect_coeffs() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let (lender, _, _, debt_reserve) = fill_pool_two(&env, &sut);
    let debt_token = &debt_reserve.token.address;
    let rwa_config = sut.rwa_config();
    rwa_config.token_admin.mint(&lender, &100_000_000);
    sut.pool
        .deposit(&lender, &rwa_config.token.address, &100_000_000);

    set_time(&env, &sut, 2 * DAY, false);

    let collat_coeff_prev = sut.pool.collat_coeff(debt_token);
    let debt_coeff_prev = sut.pool.debt_coeff(debt_token);

    sut.pool
        .withdraw(&lender, &rwa_config.token.address, &i128::MAX, &lender);

    let collat_coeff = sut.pool.collat_coeff(debt_token);
    let debt_coeff = sut.pool.debt_coeff(debt_token);

    assert_eq!(collat_coeff_prev, collat_coeff);
    assert_eq!(debt_coeff_prev, debt_coeff);
}

#[test]
fn rwa_should_affect_account_data() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, true);
    let rwa_address = &sut.rwa_config().token.address;
    let rwa_config = sut.rwa_config();
    rwa_config.token_admin.mint(&borrower, &100_000_000);
    sut.pool
        .deposit(&borrower, &rwa_config.token.address, &100_000_000);

    let account_position_prev = sut.pool.account_position(&borrower);

    set_time(&env, &sut, 2 * DAY, false);

    sut.pool
        .withdraw(&borrower, rwa_address, &100_000, &borrower);

    let account_position = sut.pool.account_position(&borrower);

    set_time(&env, &sut, 2 * DAY + 1, false);

    let collat_token_total_supply = sut.s_token().total_supply();
    let pool_collat_token_total_supply = sut.pool.token_total_supply(&sut.s_token().address);

    let collat_token_balance = sut.s_token().balance(&borrower);
    let pool_collat_token_balance = sut.pool.token_balance(&sut.s_token().address, &borrower);

    assert_eq!(collat_token_total_supply, pool_collat_token_total_supply);
    assert_eq!(collat_token_balance, pool_collat_token_balance);

    assert!(account_position_prev.discounted_collateral > account_position.discounted_collateral);
    assert!(account_position_prev.debt < account_position.debt);
    assert!(account_position_prev.npv > account_position.npv);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn should_fail_when_bad_position_after_withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        grace_period: 1,
        user_assets_limit: 4,
        min_collat_amount: 0,
        min_debt_amount: 0,
        liquidation_protocol_fee: 0,
        ir_alpha: 143,
        ir_initial_rate: 200,
        ir_max_rate: 50_000,
        ir_scaling_coeff: 9_000,
    });

    let lender = Address::generate(&env);
    let borrower = Address::generate(&env);
    sut.reserves[0].token_admin.mint(&lender, &1_000_000_000);
    sut.reserves[1]
        .token_admin
        .mint(&borrower, &100_000_000_000);

    sut.pool
        .deposit(&lender, &sut.reserves[0].token.address, &500_000_000);
    sut.pool
        .deposit(&borrower, &sut.reserves[1].token.address, &20_000_000_000);

    sut.pool
        .borrow(&borrower, &sut.reserves[0].token.address, &50_000_000);
    sut.pool
        .borrow(&borrower, &sut.reserves[0].token.address, &39_000_000);

    sut.pool.withdraw(
        &borrower,
        &sut.reserves[1].token.address,
        &14_000_000_000,
        &borrower,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn should_fail_when_collat_lt_min_position_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 2_500,
        timestamp_window: 20,
        grace_period: 1,
        user_assets_limit: 4,
        min_collat_amount: 115_000_000,
        min_debt_amount: 0,
        liquidation_protocol_fee: 0,
        ir_alpha: 143,
        ir_initial_rate: 200,
        ir_max_rate: 50_000,
        ir_scaling_coeff: 9_000,
    });

    let lender = Address::generate(&env);
    let borrower = Address::generate(&env);
    sut.reserves[0].token_admin.mint(&lender, &1_000_000_000);
    sut.reserves[1]
        .token_admin
        .mint(&borrower, &100_000_000_000);

    sut.pool
        .deposit(&lender, &sut.reserves[0].token.address, &500_000_000);
    sut.pool
        .deposit(&borrower, &sut.reserves[1].token.address, &20_000_000_000);

    sut.pool
        .borrow(&borrower, &sut.reserves[0].token.address, &50_000_000);
    sut.pool
        .borrow(&borrower, &sut.reserves[0].token.address, &39_000_000);

    sut.pool.withdraw(
        &borrower,
        &sut.reserves[1].token.address,
        &1_000_000_000,
        &borrower,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")]
fn should_fail_in_grace_period() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, false);
    let collat_address = sut.reserves[0].token.address.clone();
    sut.pool.withdraw(&borrower, &collat_address, &1, &borrower);

    sut.pool.set_pause(&true);
    sut.pool.set_pause(&false);
    sut.pool.withdraw(&borrower, &collat_address, &1, &borrower);
}

#[test]
fn should_not_fail_after_grace_period() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, false);
    let collat_address = sut.reserves[0].token.address.clone();
    let pause_info = sut.pool.pause_info();
    let start = env.ledger().timestamp();
    let gap = 500;

    let s_token_before = sut.reserves[0].s_token().balance(&borrower);
    sut.pool.withdraw(&borrower, &collat_address, &1, &borrower);
    let s_token_after = sut.reserves[0].debt_token().balance(&borrower);
    assert!(s_token_after < s_token_before);

    sut.pool.set_pause(&true);
    set_time(&env, &sut, start + gap, false);
    sut.pool.set_pause(&false);
    set_time(
        &env,
        &sut,
        start + gap + pause_info.grace_period_secs,
        false,
    );

    let s_token_before = sut.reserves[0].s_token().balance(&borrower);
    sut.pool.withdraw(&borrower, &collat_address, &1, &borrower);
    let s_token_after = sut.reserves[0].debt_token().balance(&borrower);
    assert!(s_token_after < s_token_before);
}
