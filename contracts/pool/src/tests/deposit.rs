use crate::tests::sut::{fill_pool, init_pool, DAY};
use crate::*;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, Events};
use soroban_sdk::{symbol_short, vec, IntoVal, Symbol};
use tests::sut::set_time;

#[test]
fn should_require_authorized_caller() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);
    let sut = init_pool(&env, false);
    let token_address = sut.token().address.clone();

    sut.token_admin().mint(&user, &1_000_000_000);
    sut.pool.deposit(&user, &token_address, &1_000_000_000);

    assert_eq!(
        env.auths().pop().map(|f| f.1.function).unwrap(),
        AuthorizedFunction::Contract((
            sut.pool.address.clone(),
            symbol_short!("deposit"),
            (user.clone(), token_address, 1_000_000_000i128).into_val(&env)
        )),
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn should_fail_when_pool_paused() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);
    let sut = init_pool(&env, false);
    let token_address = sut.token().address.clone();

    sut.pool.set_pause(&true);
    sut.pool.deposit(&user, &token_address, &1);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #302)")]
fn should_fail_when_invalid_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);
    let sut = init_pool(&env, false);
    let token_address = sut.token().address.clone();

    sut.pool.deposit(&user, &token_address, &-1);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #100)")]
fn should_fail_when_reserve_deactivated() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);
    let sut = init_pool(&env, false);
    let token_address = sut.token().address.clone();

    sut.pool.set_reserve_status(&token_address, &false);
    sut.pool.deposit(&user, &token_address, &1);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #4)")]
fn should_fail_when_liquidity_cap_exceeded() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let token = &sut.reserves[0].token;
    let token_admin = &sut.reserves[0].token_admin;
    let decimals = token.decimals();

    let user = Address::generate(&env);
    let initial_balance = 1_000_000_000 * 10i128.pow(decimals);

    token_admin.mint(&user, &initial_balance);
    assert_eq!(token.balance(&user), initial_balance);

    let deposit_amount = initial_balance;
    sut.pool.deposit(&user, &token.address, &deposit_amount);
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

    let user_config = sut.pool.user_configuration(&user);
    let reserve = sut.pool.get_reserve(&token_address).unwrap();

    assert_eq!(
        user_config.is_using_as_collateral(&env, reserve.get_id()),
        true
    );
    assert_eq!(user_config.total_assets(), 1);
}

#[test]
fn should_change_balances() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);
    let sut = init_pool(&env, false);
    let token_address = sut.token().address.clone();

    sut.token_admin().mint(&user, &10_000_000_000);
    set_time(&env, &sut, 2 * DAY, false);

    sut.pool.deposit(&user, &token_address, &3_000_000_000);

    let stoken_underlying_balance = sut.pool.token_balance(&sut.token().address, &sut.s_token().address);
    let user_balance = sut.token().balance(&user);
    let user_stoken_balance = sut.s_token().balance(&user);

    assert_eq!(stoken_underlying_balance, 3_000_000_000);
    assert_eq!(user_balance, 7_000_000_000);
    assert_eq!(user_stoken_balance, 3_000_000_000);
}

#[test]
fn should_affect_coeffs() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (lender, _, debt_config) = fill_pool(&env, &sut, true);
    let debt_token = &debt_config.token.address;

    set_time(&env, &sut, 2 * DAY, false);

    let collat_coeff_prev = sut.pool.collat_coeff(&debt_token);
    let debt_coeff_prev = sut.pool.debt_coeff(&debt_token);

    sut.pool
        .deposit(&lender, &sut.reserves[1].token.address, &100_000_000);

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

    let account_position_prev = sut.pool.account_position(&borrower);
    let collat_token = &sut.reserves[0];

    sut.pool
        .deposit(&borrower, &collat_token.token.address, &2_000_000);

    let account_position = sut.pool.account_position(&borrower);

    let collat_token_total_supply = collat_token.s_token().total_supply();
    let pool_collat_token_total_supply =
        sut.pool.token_total_supply(&collat_token.s_token().address);
    let collat_token_balance = collat_token.s_token().balance(&borrower);
    let pool_collat_token_balance = sut
        .pool
        .token_balance(&collat_token.s_token().address, &borrower);

    assert_eq!(collat_token_total_supply, pool_collat_token_total_supply);
    assert_eq!(collat_token_balance, pool_collat_token_balance);

    assert!(account_position_prev.discounted_collateral < account_position.discounted_collateral);
    assert!(account_position_prev.debt == account_position.debt);
    assert!(account_position_prev.npv < account_position.npv);
}

#[test]
fn should_emit_events() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let user = Address::generate(&env);
    let token_address = sut.token().address.clone();

    sut.token_admin().mint(&user, &10_000_000_000);
    assert_eq!(sut.token().balance(&user), 10_000_000_000);

    sut.pool.deposit(&user, &token_address, &5_000_000_000);

    let mut events = env.events().all();
    let event = events.pop_back_unchecked();

    assert_eq!(
        vec![&env, event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (
                    Symbol::new(&env, "reserve_used_as_coll_enabled"),
                    user.clone()
                )
                    .into_val(&env),
                (token_address.clone()).into_val(&env)
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
                (Symbol::new(&env, "deposit"), user.clone()).into_val(&env),
                (token_address, 5_000_000_000i128).into_val(&env)
            ),
        ]
    );
}

#[test]
fn rwa_change_balances() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env, false);
    let (lender, borrower, _) = fill_pool(&env, &sut, true);
    let rwa_reserve_config = sut.rwa_config();
    rwa_reserve_config.token_admin.mint(&lender, &1_000_000_000);
    rwa_reserve_config
        .token_admin
        .mint(&borrower, &1_000_000_000);

    sut.pool
        .deposit(&borrower, &rwa_reserve_config.token.address, &1_000_000_000);

    let borrower_balance_after = rwa_reserve_config.token.balance(&borrower);
    let pool_balance_after = rwa_reserve_config.token.balance(&sut.pool.address);
    assert_eq!(borrower_balance_after, 0);
    assert_eq!(pool_balance_after, 1_000_000_000);

    sut.pool
        .deposit(&lender, &rwa_reserve_config.token.address, &1_000_000_000);

    let lender_balance_after = rwa_reserve_config.token.balance(&lender);
    let pool_balance_after = rwa_reserve_config.token.balance(&sut.pool.address);
    assert_eq!(lender_balance_after, 0);
    assert_eq!(pool_balance_after, 2_000_000_000);
}

#[test]
fn rwa_should_not_affect_coeffs() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (lender, _, debt_config) = fill_pool(&env, &sut, true);
    let rwa_reserve_config = sut.rwa_config();
    rwa_reserve_config.token_admin.mint(&lender, &1_000_000_000);
    let debt_token = &debt_config.token.address;

    set_time(&env, &sut, 2 * DAY, false);

    let collat_coeff_prev = sut.pool.collat_coeff(&debt_token);
    let debt_coeff_prev = sut.pool.debt_coeff(&debt_token);

    sut.pool
        .deposit(&lender, &rwa_reserve_config.token.address, &100_000_000);

    let collat_coeff = sut.pool.collat_coeff(&debt_token);
    let debt_coeff = sut.pool.debt_coeff(&debt_token);

    assert_eq!(collat_coeff_prev, collat_coeff);
    assert_eq!(debt_coeff_prev, debt_coeff);
}

#[test]
fn rwa_should_affect_account_data() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, true);
    let rwa_reserve_config = sut.rwa_config();
    rwa_reserve_config
        .token_admin
        .mint(&borrower, &1_000_000_000);

    let borrower_position_prev = sut.pool.account_position(&borrower);

    sut.pool
        .deposit(&borrower, &rwa_reserve_config.token.address, &2_000_000);

    let borrower_position = sut.pool.account_position(&borrower);

    assert!(borrower_position_prev.discounted_collateral < borrower_position.discounted_collateral);
    assert!(borrower_position_prev.debt == borrower_position.debt);
    assert!(borrower_position_prev.npv < borrower_position.npv);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #4)")]
fn rwa_fail_when_exceed_assets_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, true);

    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 0,
        timestamp_window: 20,
        grace_period: 1,
        user_assets_limit: 2,
        min_collat_amount: 0,
        min_debt_amount: 0,
        liquidation_protocol_fee: 0,
        ir_alpha: 143,
            ir_initial_rate: 200,
            ir_max_rate: 50_000,
            ir_scaling_coeff: 9_000,
    });

    sut.pool
        .deposit(&borrower, &sut.reserves[2].token.address, &1_000_000_000);
}

#[test]
fn should_not_fail_in_grace_period() {
    let env = Env::default();
    env.mock_all_auths();

    let user = Address::generate(&env);
    let sut = init_pool(&env, false);
    let token_address = sut.token().address.clone();

    sut.token_admin().mint(&user, &10_000_000_000);
    set_time(&env, &sut, 2 * DAY, false);

    sut.pool.deposit(&user, &token_address, &3_000_000_000);

    sut.pool.set_pause(&true);
    sut.pool.set_pause(&false);

    sut.pool.deposit(&user, &token_address, &3_000_000_000);

    let stoken_underlying_balance = sut.pool.token_balance(&sut.token().address, &sut.s_token().address);
    let user_balance = sut.token().balance(&user);
    let user_stoken_balance = sut.s_token().balance(&user);

    assert_eq!(stoken_underlying_balance, 6_000_000_000);
    assert_eq!(user_balance, 4_000_000_000);
    assert_eq!(user_stoken_balance, 6_000_000_000);
}
