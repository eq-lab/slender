use crate::rate::{calc_interest_rate, calc_next_accrued_rate};
use crate::*;
use common::FixedI128;
use debt_token_interface::DebtTokenClient;
use price_feed_interface::PriceFeedClient;
use s_token_interface::STokenClient;
use soroban_sdk::symbol_short;
use soroban_sdk::testutils::{Address as _, Events, Ledger, MockAuth, MockAuthInvoke};
use soroban_sdk::{
    token::AdminClient as TokenAdminClient, token::Client as TokenClient, vec, IntoVal, Symbol,
};

use super::sut::{ReserveConfig, Sut};

extern crate std;

mod s_token {
    soroban_sdk::contractimport!(file = "../target/wasm32-unknown-unknown/release/s_token.wasm");
}

mod debt_token {
    soroban_sdk::contractimport!(file = "../target/wasm32-unknown-unknown/release/debt_token.wasm");
}

mod price_feed {
    soroban_sdk::contractimport!(
        file = "../target/wasm32-unknown-unknown/release/price_feed_mock.wasm"
    );
}

const DAY: u64 = 24 * 60 * 60;

fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let stellar_asset_contract = e.register_stellar_asset_contract(admin.clone());
    (
        TokenClient::new(e, &stellar_asset_contract),
        TokenAdminClient::new(e, &stellar_asset_contract),
    )
}

fn create_pool_contract<'a>(e: &Env, admin: &Address) -> LendingPoolClient<'a> {
    let client = LendingPoolClient::new(e, &e.register_contract(None, LendingPool));
    let treasury = Address::random(e);
    client.initialize(
        &admin,
        &treasury,
        &IRParams {
            alpha: 143,
            initial_rate: 200,
            max_rate: 50_000,
            scaling_coeff: 9_000,
        },
    );
    client
}

fn create_s_token_contract<'a>(
    e: &Env,
    pool: &Address,
    underlying_asset: &Address,
) -> STokenClient<'a> {
    let client = STokenClient::new(&e, &e.register_contract_wasm(None, s_token::WASM));

    client.initialize(
        &"SToken".into_val(e),
        &"STOKEN".into_val(e),
        &pool,
        &underlying_asset,
    );

    client
}

fn create_debt_token_contract<'a>(
    e: &Env,
    pool: &Address,
    underlying_asset: &Address,
) -> DebtTokenClient<'a> {
    let client: DebtTokenClient<'_> =
        DebtTokenClient::new(&e, &e.register_contract_wasm(None, debt_token::WASM));

    client.initialize(
        &"DebtToken".into_val(e),
        &"DTOKEN".into_val(e),
        &pool,
        &underlying_asset,
    );

    client
}

fn create_price_feed_contract<'a>(e: &Env) -> PriceFeedClient<'a> {
    PriceFeedClient::new(&e, &e.register_contract_wasm(None, price_feed::WASM))
}

pub(crate) fn init_pool<'a>(env: &Env) -> Sut<'a> {
    let admin = Address::random(&env);
    let token_admin = Address::random(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin);
    let price_feed: PriceFeedClient<'_> = create_price_feed_contract(&env);

    let reserves: std::vec::Vec<ReserveConfig<'a>> = (0..3)
        .map(|_i| {
            let (token, token_admin_client) = create_token_contract(&env, &token_admin);
            let debt_token = create_debt_token_contract(&env, &pool.address, &token.address);
            let s_token = create_s_token_contract(&env, &pool.address, &token.address);
            let decimals = s_token.decimals();
            assert!(pool.get_reserve(&s_token.address).is_none());

            env.budget().reset_default();

            pool.init_reserve(
                &token.address,
                &InitReserveInput {
                    s_token_address: s_token.address.clone(),
                    debt_token_address: debt_token.address.clone(),
                },
            );

            let liq_bonus = 11000; //110%
            let liq_cap = 100_000_000 * 10_i128.pow(decimals); // 100M
            let util_cap = 9000; //90%
            let discount = 6000; //60%

            pool.configure_as_collateral(
                &token.address,
                &CollateralParamsInput {
                    liq_bonus,
                    liq_cap,
                    util_cap,
                    discount,
                },
            );

            pool.enable_borrowing_on_reserve(&token.address, &true);

            let reserve = pool.get_reserve(&token.address);
            assert_eq!(reserve.is_some(), true);

            let reserve_config = reserve.unwrap().configuration;
            assert_eq!(reserve_config.borrowing_enabled, true);
            assert_eq!(reserve_config.liq_bonus, liq_bonus);
            assert_eq!(reserve_config.liq_cap, liq_cap);
            assert_eq!(reserve_config.util_cap, util_cap);
            assert_eq!(reserve_config.discount, discount);

            pool.set_price_feed(
                &price_feed.address,
                &soroban_sdk::vec![env, token.address.clone()],
            );

            let pool_price_feed = pool.price_feed(&token.address);
            assert_eq!(pool_price_feed, Some(price_feed.address.clone()));

            ReserveConfig {
                token,
                token_admin: token_admin_client,
                s_token,
                debt_token,
            }
        })
        .collect();

    env.budget().reset_default();

    Sut {
        pool,
        price_feed,
        pool_admin: admin,
        token_admin: token_admin,
        reserves,
    }
}

#[test]
fn init_reserve() {
    let env = Env::default();

    let admin = Address::random(&env);
    let token_admin = Address::random(&env);

    let (underlying_token, _) = create_token_contract(&env, &token_admin);
    let (debt_token, _) = create_token_contract(&env, &token_admin);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin);
    let s_token = create_s_token_contract(&env, &pool.address, &underlying_token.address);
    assert!(pool.get_reserve(&underlying_token.address).is_none());

    let init_reserve_input = InitReserveInput {
        s_token_address: s_token.address.clone(),
        debt_token_address: debt_token.address.clone(),
    };

    assert_eq!(
        pool.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &pool.address,
                fn_name: "init_reserve",
                args: (&underlying_token.address, init_reserve_input.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .init_reserve(&underlying_token.address, &init_reserve_input),
        ()
    );

    let reserve = pool.get_reserve(&underlying_token.address).unwrap();

    assert!(pool.get_reserve(&underlying_token.address).is_some());
    assert_eq!(init_reserve_input.s_token_address, reserve.s_token_address);
    assert_eq!(
        init_reserve_input.debt_token_address,
        reserve.debt_token_address
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn init_reserve_second_time() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let init_reserve_input = InitReserveInput {
        s_token_address: sut.s_token().address.clone(),
        debt_token_address: sut.debt_token().address.clone(),
    };

    //TODO: check error after soroban fix
    sut.pool
        .init_reserve(&sut.token().address, &init_reserve_input);

    // assert_eq!(
    //     sut.pool
    //         .try_init_reserve(&sut.token().address, &init_reserve_input)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::ReserveAlreadyInitialized
    // )
}

#[test]
fn init_reserve_when_pool_not_initialized() {
    let env = Env::default();

    let admin = Address::random(&env);
    let token_admin = Address::random(&env);

    let (underlying_token, _) = create_token_contract(&env, &token_admin);
    let (debt_token, _) = create_token_contract(&env, &token_admin);

    let pool: LendingPoolClient<'_> =
        LendingPoolClient::new(&env, &env.register_contract(None, LendingPool));
    let s_token = create_s_token_contract(&env, &pool.address, &underlying_token.address);
    assert!(pool.get_reserve(&underlying_token.address).is_none());

    let init_reserve_input = InitReserveInput {
        s_token_address: s_token.address.clone(),
        debt_token_address: debt_token.address.clone(),
    };

    assert_eq!(
        pool.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &pool.address,
                fn_name: "init_reserve",
                args: (&underlying_token.address, init_reserve_input.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .try_init_reserve(&underlying_token.address, &init_reserve_input)
        .unwrap_err()
        .unwrap(),
        Error::Uninitialized
    );
}

#[test]
fn set_ir_params() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let ir_params_input = IRParams {
        alpha: 144,
        initial_rate: 201,
        max_rate: 50_001,
        scaling_coeff: 9_001,
    };

    sut.pool.set_ir_params(&ir_params_input);

    let ir_params = sut.pool.ir_params().unwrap();

    assert_eq!(ir_params_input.alpha, ir_params.alpha);
    assert_eq!(ir_params_input.initial_rate, ir_params.initial_rate);
    assert_eq!(ir_params_input.max_rate, ir_params.max_rate);
    assert_eq!(ir_params_input.scaling_coeff, ir_params.scaling_coeff);
}

#[test]
fn withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let (lender, _borrower, debt_config) = fill_pool(&env, &sut);
    let debt_token = &debt_config.token.address;

    env.ledger().with_mut(|li| {
        li.timestamp = 60 * DAY;
    });

    let lender_s_token_balance = debt_config.s_token.balance(&lender);
    let s_token_supply = debt_config.s_token.total_supply();
    assert_eq!(s_token_supply, 100000000);
    assert_eq!(lender_s_token_balance, 100000000);

    let withdraw_amount = 1_000_000;
    sut.pool
        .withdraw(&lender, debt_token, &withdraw_amount, &lender);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&debt_config.s_token.address);

    let lender_underlying_balance = debt_config.token.balance(&lender);
    let lender_s_token_balance = debt_config.s_token.balance(&lender);
    let s_token_supply = debt_config.s_token.total_supply();

    assert_eq!(lender_underlying_balance, 901000000);
    assert_eq!(lender_s_token_balance, 99002451);
    assert_eq!(s_token_supply, 99002451);
    assert_eq!(s_token_underlying_supply, 59_000_000);
}

#[test]
fn withdraw_full() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let (lender, _lender, _borrower, debt_config) = fill_pool_two(&env, &sut);
    let debt_token = &debt_config.token.address;

    env.ledger().with_mut(|li| {
        li.timestamp = 60 * DAY;
    });

    let lender_s_token_balance = debt_config.s_token.balance(&lender);
    let s_token_supply = debt_config.s_token.total_supply();
    assert_eq!(s_token_supply, 200000000);
    assert_eq!(lender_s_token_balance, 100000000);

    let withdraw_amount = i128::MAX;
    sut.pool
        .withdraw(&lender, debt_token, &withdraw_amount, &lender);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&debt_config.s_token.address);

    let lender_underlying_balance = debt_config.token.balance(&lender);
    let lender_s_token_balance = debt_config.s_token.balance(&lender);
    let s_token_supply = debt_config.s_token.total_supply();

    assert_eq!(lender_underlying_balance, 1000081366);
    assert_eq!(lender_s_token_balance, 0);
    assert_eq!(s_token_supply, 100000000);
    assert_eq!(s_token_underlying_supply, 59_918_634);
}

#[test]
fn withdraw_base() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let user1 = Address::random(&env);
    let user2 = Address::random(&env);

    let initial_balance = 1_000_000_000;
    sut.token_admin().mint(&user1, &1_000_000_000);
    assert_eq!(sut.token().balance(&user1), initial_balance);

    let deposit_amount = 10000;
    sut.pool
        .deposit(&user1, &sut.token().address, &deposit_amount);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.s_token().address);

    assert_eq!(sut.s_token().balance(&user1), deposit_amount);
    assert_eq!(
        sut.token().balance(&user1),
        initial_balance - deposit_amount
    );
    assert_eq!(sut.token().balance(&sut.s_token().address), deposit_amount);
    assert_eq!(s_token_underlying_supply, 10_000);

    let amount_to_withdraw = 3500;
    sut.pool
        .withdraw(&user1, &sut.token().address, &amount_to_withdraw, &user2);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.s_token().address);

    assert_eq!(sut.token().balance(&user2), amount_to_withdraw);
    assert_eq!(
        sut.s_token().balance(&user1),
        deposit_amount - amount_to_withdraw
    );
    assert_eq!(
        sut.token().balance(&sut.s_token().address),
        deposit_amount - amount_to_withdraw
    );
    assert_eq!(s_token_underlying_supply, 6_500);

    let withdraw_event = env.events().all().pop_back_unchecked();
    assert_eq!(
        vec![&env, withdraw_event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (symbol_short!("withdraw"), &user1).into_val(&env),
                (&user2, &sut.token().address, amount_to_withdraw).into_val(&env)
            ),
        ]
    );

    sut.pool
        .withdraw(&user1, &sut.token().address, &i128::MAX, &user2);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.s_token().address);

    assert_eq!(sut.token().balance(&user2), deposit_amount);
    assert_eq!(sut.s_token().balance(&user1), 0);
    assert_eq!(sut.token().balance(&sut.s_token().address), 0);
    assert_eq!(s_token_underlying_supply, 0);

    let withdraw_event = env.events().all().pop_back_unchecked();
    assert_eq!(
        vec![&env, withdraw_event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (symbol_short!("withdraw"), &user1).into_val(&env),
                (
                    &user2,
                    sut.token().address.clone(),
                    deposit_amount - amount_to_withdraw
                )
                    .into_val(&env)
            ),
        ]
    );

    let coll_disabled_event = env
        .events()
        .all()
        .get(env.events().all().len() - 4)
        .unwrap();
    assert_eq!(
        vec![&env, coll_disabled_event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "reserve_used_as_coll_disabled"), &user1).into_val(&env),
                (sut.token().address.clone()).into_val(&env)
            ),
        ]
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn withdraw_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let _token1 = &sut.reserves[0].token;
    let token2 = &sut.reserves[1].token;
    let token2_admin = &sut.reserves[1].token_admin;

    let user1 = Address::random(&env);
    token2_admin.mint(&user1, &1000);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    sut.pool.deposit(&user1, &token2.address, &1000);

    let withdraw_amount = 0;

    sut.pool
        .withdraw(&user1, &token2.address, &withdraw_amount, &user1);
    //TODO: check error after soroban fix
    // assert_eq!(
    //     sut.pool
    //         .try_withdraw(&user1, &token1.address, &withdraw_amount, &user1),
    //     Err(Ok(Error::InvalidAmount))
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn withdraw_more_than_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let token = &sut.reserves[0].token;
    let token_admin = &sut.reserves[0].token_admin;

    let user1 = Address::random(&env);

    let initial_balance = 1_000_000_000;
    token_admin.mint(&user1, &1_000_000_000);
    assert_eq!(token.balance(&user1), initial_balance);

    env.budget().reset_unlimited();

    let deposit_amount = 1000;
    sut.pool.deposit(&user1, &token.address, &deposit_amount);

    let withdraw_amount = 2000;

    //TODO: check error after soroban fix
    sut.pool
        .withdraw(&user1, &token.address, &withdraw_amount, &user1);

    // assert_eq!(
    //     sut.pool
    //         .try_withdraw(&user1, &token.address, &withdraw_amount, &user1)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::NotEnoughAvailableUserBalance
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn withdraw_unknown_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let user1 = Address::random(&env);
    let unknown_asset = &sut.reserves[0].debt_token.address;

    //TODO: check error after soroban fix
    let withdraw_amount = 1000;
    sut.pool
        .withdraw(&user1, unknown_asset, &withdraw_amount, &user1);

    // assert_eq!(
    //     sut.pool
    //         .try_withdraw(&user1, unknown_asset, &withdraw_amount, &user1)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::NoReserveExistForAsset
    // )
}

#[test]
fn withdraw_non_active_reserve() {
    //TODO: implement when it possible
}

#[test]
fn deposit() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let token = &sut.reserves[0].token;
    let token_admin = &sut.reserves[0].token_admin;
    let s_token = &sut.reserves[0].s_token;

    for i in 0..10 {
        let user = Address::random(&env);
        let initial_balance = 1_000_000_000;
        token_admin.mint(&user, &1_000_000_000);
        assert_eq!(token.balance(&user), initial_balance);

        let deposit_amount = 10_000;
        let lender_accrued_rate = Some(FixedI128::ONE.into_inner() + i * 100_000_000);

        assert_eq!(
            sut.pool
                .set_accrued_rates(&token.address, &lender_accrued_rate, &None),
            ()
        );
        let collat_coeff = sut.pool.collat_coeff(&token.address);
        sut.pool.deposit(&user, &token.address, &deposit_amount);

        assert_eq!(
            s_token.balance(&user),
            deposit_amount * FixedI128::ONE.into_inner() / collat_coeff
        );
        assert_eq!(token.balance(&user), initial_balance - deposit_amount);

        let last = env.events().all().pop_back_unchecked();
        assert_eq!(
            vec![&env, last],
            vec![
                &env,
                (
                    sut.pool.address.clone(),
                    (Symbol::new(&env, "reserve_used_as_coll_enabled"), user).into_val(&env),
                    (token.address.clone()).into_val(&env)
                ),
            ]
        );
    }
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn deposit_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let user1 = Address::random(&env);

    //TODO: check error after soroban fix
    let deposit_amount = 0;
    sut.pool
        .deposit(&user1, &sut.reserves[0].token.address, &deposit_amount);

    // assert_eq!(
    //     sut.pool
    //         .try_deposit(&user1, &sut.reserves[0].token.address, &deposit_amount,)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::InvalidAmount
    // )
}

#[test]
fn deposit_non_active_reserve() {
    //TODO: implement when possible
}

#[test]
fn deposit_frozen_() {
    //TODO: implement when possible
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn deposit_should_fail_when_exceeded_liq_cap() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let token = &sut.reserves[0].token;
    let token_admin = &sut.reserves[0].token_admin;
    let s_token = &sut.reserves[0].s_token;
    let decimals = s_token.decimals();

    let user = Address::random(&env);
    let initial_balance = 1_000_000_000 * 10i128.pow(decimals);

    token_admin.mint(&user, &initial_balance);
    assert_eq!(token.balance(&user), initial_balance);

    let deposit_amount = initial_balance;
    sut.pool.deposit(&user, &token.address, &deposit_amount);

    // assert_eq!(
    //     sut.pool
    //         .try_deposit(&user, &token.address, &deposit_amount)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::LiqCapExceeded
    // )
}

#[test]
fn borrow() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let initial_amount: i128 = 1_000_000_000;
    let lender = Address::random(&env);
    let borrower = Address::random(&env);

    for r in sut.reserves.iter() {
        r.token_admin.mint(&lender, &initial_amount);
        assert_eq!(r.token.balance(&lender), initial_amount);

        r.token_admin.mint(&borrower, &initial_amount);
        assert_eq!(r.token.balance(&borrower), initial_amount);
    }

    //TODO: optimize gas
    env.budget().reset_unlimited();

    //lender deposit all tokens
    let deposit_amount = 100_000_000;
    for r in sut.reserves.iter() {
        let pool_balance = r.token.balance(&r.s_token.address);
        sut.pool.deposit(&lender, &r.token.address, &deposit_amount);
        assert_eq!(r.s_token.balance(&lender), deposit_amount);
        assert_eq!(
            r.token.balance(&r.s_token.address),
            pool_balance + deposit_amount
        );
    }

    //borrower deposit first token and borrow second token
    sut.pool
        .deposit(&borrower, &sut.reserves[0].token.address, &deposit_amount);
    assert_eq!(sut.reserves[0].s_token.balance(&borrower), deposit_amount);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

    assert_eq!(s_token_underlying_supply, 100_000_000);

    //borrower borrow second token
    let borrow_asset = sut.reserves[1].token.address.clone();
    let borrow_amount = 10_000;
    let pool_balance_before = sut.reserves[1]
        .token
        .balance(&sut.reserves[1].s_token.address);

    let borrower_balance_before = sut.reserves[1].token.balance(&borrower);
    sut.pool.borrow(&borrower, &borrow_asset, &borrow_amount);
    assert_eq!(
        sut.reserves[1].token.balance(&borrower),
        borrower_balance_before + borrow_amount
    );

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

    let pool_balance = sut.reserves[1]
        .token
        .balance(&sut.reserves[1].s_token.address);
    let debt_token_balance = sut.reserves[1].debt_token.balance(&borrower);
    assert_eq!(
        pool_balance + borrow_amount,
        pool_balance_before,
        "Pool balance"
    );
    assert_eq!(debt_token_balance, borrow_amount, "Debt token balance");
    assert_eq!(s_token_underlying_supply, 99_990_000);
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn borrow_utilization_exceeded() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let initial_amount: i128 = 1_000_000_000;
    let lender = Address::random(&env);
    let borrower = Address::random(&env);

    sut.reserves[0].token_admin.mint(&lender, &initial_amount);
    sut.reserves[1].token_admin.mint(&borrower, &initial_amount);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let deposit_amount = 1_000_000_000;

    sut.pool
        .deposit(&lender, &sut.reserves[0].token.address, &deposit_amount);

    sut.pool
        .deposit(&borrower, &sut.reserves[1].token.address, &deposit_amount);

    sut.pool
        .borrow(&borrower, &sut.reserves[0].token.address, &990_000_000);

    // assert_eq!(
    //     sut.pool
    //         .try_borrow(&borrower, &sut.reserves[0].token.address, &990_000_000)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::UtilizationCapExceeded
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn borrow_user_confgig_not_exists() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let borrower = Address::random(&env);

    //TODO: check error after soroban fix
    let borrow_amount = 0;
    sut.pool
        .borrow(&borrower, &sut.reserves[0].token.address, &borrow_amount);
    // assert_eq!(
    //     sut.pool
    //         .try_borrow(&borrower, &sut.reserves[0].token.address, &borrow_amount)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::UserConfigNotExists
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn borrow_collateral_is_zero() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let lender = Address::random(&env);
    let borrower = Address::random(&env);

    let initial_amount = 1_000_000_000;
    for r in sut.reserves.iter() {
        r.token_admin.mint(&borrower, &initial_amount);
        assert_eq!(r.token.balance(&borrower), initial_amount);
        r.token_admin.mint(&lender, &initial_amount);
        assert_eq!(r.token.balance(&lender), initial_amount);
    }

    let deposit_amount = 1000;

    env.budget().reset_unlimited();

    sut.pool
        .deposit(&lender, &sut.reserves[0].token.address, &deposit_amount);

    sut.pool
        .deposit(&borrower, &sut.reserves[1].token.address, &deposit_amount);

    sut.pool.withdraw(
        &borrower,
        &sut.reserves[1].token.address,
        &deposit_amount,
        &borrower,
    );

    let borrow_amount = 100;
    sut.pool
        .borrow(&borrower, &sut.reserves[0].token.address, &borrow_amount)

    //TODO: check error after fix
    // assert_eq!(
    //     sut.pool
    //         .try_borrow(&borrower, &sut.reserves[0].token.address, &borrow_amount)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::CollateralNotCoverNewBorrow
    // )
}

#[test]
fn borrow_no_active_reserve() {
    //TODO: implement
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn borrow_collateral_not_cover_new_debt() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let lender = Address::random(&env);
    let borrower = Address::random(&env);

    let initial_amount = 1_000_000_000;
    for r in sut.reserves.iter() {
        r.token_admin.mint(&borrower, &initial_amount);
        assert_eq!(r.token.balance(&borrower), initial_amount);
        r.token_admin.mint(&lender, &initial_amount);
        assert_eq!(r.token.balance(&lender), initial_amount);
    }

    let borrower_deposit_amount = 500;
    let lender_deposit_amount = 2000;

    //TODO: optimize gas
    env.budget().reset_unlimited();

    sut.pool.deposit(
        &lender,
        &sut.reserves[0].token.address,
        &lender_deposit_amount,
    );

    sut.pool.deposit(
        &borrower,
        &sut.reserves[1].token.address,
        &borrower_deposit_amount,
    );

    //TODO: check error after soroban fix
    let borrow_amount = 1000;
    sut.pool
        .borrow(&borrower, &sut.reserves[0].token.address, &borrow_amount);

    // assert_eq!(
    //     sut.pool
    //         .try_borrow(&borrower, &sut.reserves[0].token.address, &borrow_amount)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::CollateralNotCoverNewBorrow
    // )
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn borrow_disabled_for_borrowing_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let initial_amount: i128 = 1_000_000_000;
    let lender = Address::random(&env);
    let borrower = Address::random(&env);

    for r in sut.reserves.iter() {
        r.token_admin.mint(&lender, &initial_amount);
        assert_eq!(r.token.balance(&lender), initial_amount);

        r.token_admin.mint(&borrower, &initial_amount);
        assert_eq!(r.token.balance(&borrower), initial_amount);
    }

    env.budget().reset_unlimited();

    //lender deposit all tokens
    let deposit_amount = 100_000_000;
    for r in sut.reserves.iter() {
        let pool_balance = r.token.balance(&r.s_token.address);
        sut.pool.deposit(&lender, &r.token.address, &deposit_amount);
        assert_eq!(r.s_token.balance(&lender), deposit_amount);
        assert_eq!(
            r.token.balance(&r.s_token.address),
            pool_balance + deposit_amount
        );
    }

    //borrower deposit first token and borrow second token
    sut.pool
        .deposit(&borrower, &sut.reserves[0].token.address, &deposit_amount);
    assert_eq!(sut.reserves[0].s_token.balance(&borrower), deposit_amount);

    //borrower borrow second token
    let borrow_asset = sut.reserves[1].token.address.clone();
    let borrow_amount = 10_000;

    //disable second token for borrowing
    sut.pool.enable_borrowing_on_reserve(&borrow_asset, &false);
    let reserve = sut.pool.get_reserve(&borrow_asset);
    assert_eq!(reserve.unwrap().configuration.borrowing_enabled, false);

    //TODO: check error after soroban fix
    sut.pool.borrow(&borrower, &borrow_asset, &borrow_amount);

    // assert_eq!(
    //     sut.pool
    //         .try_borrow(&borrower, &borrow_asset, &borrow_amount)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::BorrowingNotEnabled
    // );
}

#[test]
fn set_price_feed() {
    let env = Env::default();

    let admin = Address::random(&env);
    let asset_1 = Address::random(&env);
    let asset_2 = Address::random(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin);
    let price_feed: PriceFeedClient<'_> = create_price_feed_contract(&env);
    let assets = vec![&env, asset_1.clone(), asset_2.clone()];

    assert!(pool.price_feed(&asset_1.clone()).is_none());
    assert!(pool.price_feed(&asset_2.clone()).is_none());

    assert_eq!(
        pool.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &pool.address,
                fn_name: "set_price_feed",
                args: (&price_feed.address, assets.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_price_feed(&price_feed.address, &assets.clone()),
        ()
    );

    assert_eq!(pool.price_feed(&asset_1).unwrap(), price_feed.address);
    assert_eq!(pool.price_feed(&asset_2).unwrap(), price_feed.address);
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn test_liquidate_error_good_position() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env);
    let liquidator = Address::random(&env);
    let user = Address::random(&env);
    let token = &sut.reserves[0].token;
    let token_admin = &sut.reserves[0].token_admin;
    token_admin.mint(&user, &1_000_000_000);

    env.budget().reset_unlimited();

    sut.pool.deposit(&user, &token.address, &1_000_000_000);

    let position = sut.pool.account_position(&user);
    assert!(position.npv > 0, "test configuration");

    //TODO: check error after soroban fix
    sut.pool.liquidate(&liquidator, &user, &false);

    // assert_eq!(
    //     sut.pool
    //         .try_liquidate(&liquidator, &user, &false)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::GoodPosition
    // );
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn test_liquidate_error_not_enough_collateral() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let liquidator = Address::random(&env);
    let borrower = Address::random(&env);
    let lender = Address::random(&env);
    let token1 = &sut.reserves[0].token;
    let token1_admin = &sut.reserves[0].token_admin;
    let token2 = &sut.reserves[1].token;
    let token2_admin = &sut.reserves[1].token_admin;

    let deposit = 1_000_000_000;
    let discount = sut
        .pool
        .get_reserve(&token1.address)
        .expect("reserve")
        .configuration
        .discount;
    let debt = FixedI128::from_percentage(discount)
        .unwrap()
        .mul_int(deposit)
        .unwrap();
    token1_admin.mint(&borrower, &deposit);
    token2_admin.mint(&lender, &deposit);
    sut.pool.deposit(&borrower, &token1.address, &deposit);
    sut.pool.deposit(&lender, &token2.address, &deposit);
    sut.pool.borrow(&borrower, &token2.address, &debt);
    sut.price_feed.set_price(
        &token2.address,
        &(10i128.pow(sut.price_feed.decimals()) * 2),
    );

    let position = sut.pool.account_position(&borrower);
    assert!(position.npv < 0, "test configuration");

    //TODO: check error after soroban fix
    sut.pool.liquidate(&liquidator, &borrower, &false);

    // assert_eq!(
    //     sut.pool
    //         .try_liquidate(&liquidator, &borrower, &false)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::NotEnoughCollateral
    // );
}

#[test]
fn test_liquidate() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let liquidator = Address::random(&env);
    let borrower = Address::random(&env);
    let lender = Address::random(&env);
    let collateral_asset = &sut.reserves[0].token;
    let collateral_asset_admin = &sut.reserves[0].token_admin;
    let debt_asset = &sut.reserves[1].token;
    let debt_asset_admin = &sut.reserves[1].token_admin;
    let deposit = 1_000_000_000;
    let discount = sut
        .pool
        .get_reserve(&collateral_asset.address)
        .expect("Reserve")
        .configuration
        .discount;
    let debt = FixedI128::from_percentage(discount)
        .unwrap()
        .mul_int(deposit)
        .unwrap();
    collateral_asset_admin.mint(&borrower, &deposit);
    debt_asset_admin.mint(&lender, &deposit);
    debt_asset_admin.mint(&liquidator, &deposit);

    sut.pool
        .deposit(&borrower, &collateral_asset.address, &deposit);
    sut.pool.deposit(&lender, &debt_asset.address, &deposit);

    sut.pool.borrow(&borrower, &debt_asset.address, &debt);

    let s_token_underlying_supply_0 = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[0].s_token.address);
    let s_token_underlying_supply_1 = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

    let position = sut.pool.account_position(&borrower);
    assert!(position.npv == 0, "test configuration");

    let debt_reserve = sut.pool.get_reserve(&debt_asset.address).expect("reserve");
    let debt_token = DebtTokenClient::new(&env, &debt_reserve.debt_token_address);
    let debt_token_supply_before = debt_token.total_supply();
    let borrower_collateral_balance_before = collateral_asset.balance(&borrower);
    let stoken = STokenClient::new(
        &env,
        &sut.pool
            .get_reserve(&collateral_asset.address)
            .expect("reserve")
            .s_token_address,
    );
    let stoken_balance_before = stoken.balance(&borrower);
    assert_eq!(s_token_underlying_supply_0, 1_000_000_000);
    assert_eq!(s_token_underlying_supply_1, 400_000_000);

    assert_eq!(sut.pool.liquidate(&liquidator, &borrower, &false), ());

    let s_token_underlying_supply_0 = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[0].s_token.address);
    let s_token_underlying_supply_1 = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

    let debt_with_penalty = FixedI128::from_percentage(debt_reserve.configuration.liq_bonus)
        .unwrap()
        .mul_int(debt)
        .unwrap();
    // assume that default price is 1.0 for both assets
    assert_eq!(collateral_asset.balance(&liquidator), debt_with_penalty);
    assert_eq!(debt_asset.balance(&liquidator), deposit - debt);
    assert_eq!(debt_asset.balance(&borrower), debt);
    assert_eq!(debt_token.balance(&borrower), 0);
    assert_eq!(debt_token.total_supply(), debt_token_supply_before - debt);
    assert_eq!(
        collateral_asset.balance(&borrower),
        borrower_collateral_balance_before
    );
    assert_eq!(
        stoken.balance(&borrower),
        stoken_balance_before - debt_with_penalty
    );
    assert_eq!(s_token_underlying_supply_0, 340_000_000);
    assert_eq!(s_token_underlying_supply_1, 1_000_000_000);
}

#[test]
fn test_liquidate_receive_stoken() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env);
    //TODO: optimize gas
    env.budget().reset_unlimited();

    let liquidator = Address::random(&env);
    let borrower = Address::random(&env);
    let lender = Address::random(&env);
    let collateral_asset = &sut.reserves[0].token;
    let collateral_asset_admin = &sut.reserves[0].token_admin;
    let debt_asset = &sut.reserves[1].token;
    let debt_asset_admin = &sut.reserves[1].token_admin;
    let deposit = 1_000_000_000;
    let discount = sut
        .pool
        .get_reserve(&collateral_asset.address)
        .expect("Reserve")
        .configuration
        .discount;
    let debt = FixedI128::from_percentage(discount)
        .unwrap()
        .mul_int(deposit)
        .unwrap();
    collateral_asset_admin.mint(&borrower, &deposit);
    debt_asset_admin.mint(&lender, &deposit);
    debt_asset_admin.mint(&liquidator, &deposit);

    sut.pool
        .deposit(&borrower, &collateral_asset.address, &deposit);
    sut.pool.deposit(&lender, &debt_asset.address, &deposit);

    sut.pool.borrow(&borrower, &debt_asset.address, &debt);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

    let position = sut.pool.account_position(&borrower);
    assert!(position.npv == 0, "test configuration");

    let debt_reserve = sut.pool.get_reserve(&debt_asset.address).expect("reserve");
    let debt_token = DebtTokenClient::new(&env, &debt_reserve.debt_token_address);
    let debt_token_supply_before = debt_token.total_supply();
    let borrower_collateral_balance_before = collateral_asset.balance(&borrower);
    let liquidator_collateral_balance_before = collateral_asset.balance(&liquidator);
    let stoken = STokenClient::new(
        &env,
        &sut.pool
            .get_reserve(&collateral_asset.address)
            .expect("reserve")
            .s_token_address,
    );
    let borrower_stoken_balance_before = stoken.balance(&borrower);
    let liquidator_stoken_balance_before = stoken.balance(&liquidator);

    assert_eq!(s_token_underlying_supply, 400_000_000);

    assert_eq!(sut.pool.liquidate(&liquidator, &borrower, &true), ());

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

    let debt_with_penalty = FixedI128::from_percentage(debt_reserve.configuration.liq_bonus)
        .unwrap()
        .mul_int(debt)
        .unwrap();
    // assume that default price is 1.0 for both assets
    assert_eq!(
        collateral_asset.balance(&liquidator),
        liquidator_collateral_balance_before
    );
    assert_eq!(debt_asset.balance(&liquidator), deposit - debt);
    assert_eq!(debt_asset.balance(&borrower), debt);
    assert_eq!(debt_token.balance(&borrower), 0);
    assert_eq!(debt_token.total_supply(), debt_token_supply_before - debt);
    assert_eq!(
        collateral_asset.balance(&borrower),
        borrower_collateral_balance_before
    );
    assert_eq!(
        stoken.balance(&borrower),
        borrower_stoken_balance_before - debt_with_penalty
    );
    assert_eq!(
        stoken.balance(&liquidator),
        liquidator_stoken_balance_before + debt_with_penalty
    );
    assert_eq!(s_token_underlying_supply, 1_000_000_000);
}

#[test]
fn liquidate_over_repay_liquidator_debt() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env);

    env.budget().reset_unlimited();

    let liquidator = Address::random(&env);
    let borrower = Address::random(&env);
    let lender = Address::random(&env);

    let reserve_1 = &sut.reserves[0];
    let reserve_2 = &sut.reserves[1];

    reserve_1.token_admin.mint(&liquidator, &2_000_000_000);
    reserve_1.token_admin.mint(&borrower, &2_000_000_000);
    reserve_2.token_admin.mint(&lender, &2_000_000_000);
    reserve_2.token_admin.mint(&liquidator, &2_000_000_000);

    sut.pool
        .deposit(&lender, &reserve_2.token.address, &2_000_000_000);
    sut.pool
        .deposit(&liquidator, &reserve_2.token.address, &1_000_000_000);
    sut.pool
        .deposit(&borrower, &reserve_1.token.address, &1_000_000_000);

    let s_token_underlying_supply_1 = sut
        .pool
        .get_stoken_underlying_balance(&reserve_1.s_token.address);
    let s_token_underlying_supply_2 = sut
        .pool
        .get_stoken_underlying_balance(&reserve_2.s_token.address);

    assert_eq!(s_token_underlying_supply_1, 1_000_000_000);
    assert_eq!(s_token_underlying_supply_2, 3_000_000_000);

    sut.pool
        .borrow(&borrower, &reserve_2.token.address, &600_000_000);
    sut.pool
        .borrow(&liquidator, &reserve_1.token.address, &200_000_000);

    let s_token_underlying_supply_1 = sut
        .pool
        .get_stoken_underlying_balance(&reserve_1.s_token.address);
    let s_token_underlying_supply_2 = sut
        .pool
        .get_stoken_underlying_balance(&reserve_2.s_token.address);

    assert_eq!(s_token_underlying_supply_1, 800_000_000);
    assert_eq!(s_token_underlying_supply_2, 2_400_000_000);

    let borrower_debt_before = reserve_2.debt_token.balance(&borrower);
    let liquidator_debt_before = reserve_1.debt_token.balance(&liquidator);

    let borrower_collat_before = reserve_1.s_token.balance(&borrower);
    let liquidator_collat_before = reserve_2.s_token.balance(&liquidator);

    assert_eq!(sut.pool.liquidate(&liquidator, &borrower, &true), ());

    let s_token_underlying_supply_1 = sut
        .pool
        .get_stoken_underlying_balance(&reserve_1.s_token.address);
    let s_token_underlying_supply_2 = sut
        .pool
        .get_stoken_underlying_balance(&reserve_2.s_token.address);

    let borrower_debt_after = reserve_2.debt_token.balance(&borrower);
    let liquidator_debt_after = reserve_1.debt_token.balance(&liquidator);

    let borrower_collat_after = reserve_1.s_token.balance(&borrower);
    let liquidator_collat_after = reserve_1.s_token.balance(&liquidator);

    // borrower borrowed 600_000_000
    assert_eq!(borrower_debt_before, 600_000_000);

    // liquidator borrowed 200_000_000
    assert_eq!(liquidator_debt_before, 200_000_000);

    // borrower deposited 1_000_000_000
    assert_eq!(borrower_collat_before, 1_000_000_000);

    // liquidator deposited 1_000_000_000
    assert_eq!(liquidator_collat_before, 1_000_000_000);

    // borrower's debt repayed
    assert_eq!(borrower_debt_after, 0);

    // liquidator's debt repayed
    assert_eq!(liquidator_debt_after, 0);

    // borrower transferred stokens: 1_000_000_000 - 660_000_000 = 340_000_000
    assert_eq!(borrower_collat_after, 340_000_000);

    // liquidator accepted stokens: 660_000_000 - 200_000_000 = 460_000_000
    assert_eq!(liquidator_collat_after, 460_000_000);

    assert_eq!(s_token_underlying_supply_1, 800_000_000);
    assert_eq!(s_token_underlying_supply_2, 3_000_000_000);
}

#[test]
fn user_operation_should_update_ar_coeffs() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let debt_asset_1 = sut.reserves[1].token.address.clone();

    let lender = Address::random(&env);
    let borrower_1 = Address::random(&env);
    let borrow_amount = 40_000_000;

    //init pool with one borrower and one lender
    let initial_amount: i128 = 1_000_000_000;
    for r in sut.reserves.iter() {
        r.token_admin.mint(&lender, &initial_amount);
        r.token_admin.mint(&borrower_1, &initial_amount);
    }

    //lender deposit all tokens
    let deposit_amount = 100_000_000;
    for r in sut.reserves.iter() {
        sut.pool.deposit(&lender, &r.token.address, &deposit_amount);
    }

    sut.pool
        .deposit(&borrower_1, &sut.reserves[0].token.address, &deposit_amount);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

    assert_eq!(s_token_underlying_supply, 100_000_000);

    // ensure that zero elapsed time doesn't change AR coefficients
    {
        let reserve_before = sut.pool.get_reserve(&debt_asset_1).unwrap();
        sut.pool.borrow(&borrower_1, &debt_asset_1, &borrow_amount);

        let s_token_underlying_supply = sut
            .pool
            .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

        let updated_reserve = sut.pool.get_reserve(&debt_asset_1).unwrap();
        assert_eq!(
            updated_reserve.lender_accrued_rate,
            reserve_before.lender_accrued_rate
        );
        assert_eq!(
            updated_reserve.borrower_accrued_rate,
            reserve_before.borrower_accrued_rate
        );
        assert_eq!(
            reserve_before.last_update_timestamp,
            updated_reserve.last_update_timestamp
        );
        assert_eq!(s_token_underlying_supply, 60_000_000);
    }

    // shift time to
    env.ledger().with_mut(|li| {
        li.timestamp = 24 * 60 * 60 // one day
    });

    //second deposit by lender of debt asset
    sut.pool.deposit(&lender, &debt_asset_1, &deposit_amount);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

    let updated = sut.pool.get_reserve(&debt_asset_1).unwrap();
    let ir_params = sut.pool.ir_params().unwrap();
    let debt_ir = calc_interest_rate(deposit_amount, borrow_amount, &ir_params).unwrap();
    let lender_ir = debt_ir
        .checked_mul(FixedI128::from_percentage(ir_params.scaling_coeff).unwrap())
        .unwrap();

    let elapsed_time = env.ledger().timestamp();

    let coll_ar = calc_next_accrued_rate(FixedI128::ONE, lender_ir, elapsed_time)
        .unwrap()
        .into_inner();
    let debt_ar = calc_next_accrued_rate(FixedI128::ONE, debt_ir, elapsed_time)
        .unwrap()
        .into_inner();

    assert_eq!(updated.lender_accrued_rate, coll_ar);
    assert_eq!(updated.borrower_accrued_rate, debt_ar);
    assert_eq!(updated.lender_ir, lender_ir.into_inner());
    assert_eq!(updated.borrower_ir, debt_ir.into_inner());
    assert_eq!(s_token_underlying_supply, 160_000_000);
}

#[test]
fn repay() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let lender = Address::random(&env);
    let borrower = Address::random(&env);
    let treasury_address = &sut.pool.treasury();
    let second_stoken_address = &sut.reserves[1].s_token.address;
    let initial_amount = 100_000_000_000;

    for r in sut.reserves.iter() {
        r.token_admin.mint(&lender, &initial_amount);
        assert_eq!(r.token.balance(&lender), initial_amount);

        r.token_admin.mint(&borrower, &initial_amount);
        assert_eq!(r.token.balance(&borrower), initial_amount);
    }

    //lender deposit all tokens
    let lending_amount = 10_000_000_000;
    for r in sut.reserves.iter() {
        let pool_balance = r.token.balance(&r.s_token.address);
        sut.pool.deposit(&lender, &r.token.address, &lending_amount);
        assert_eq!(r.s_token.balance(&lender), lending_amount);
        assert_eq!(
            r.token.balance(&r.s_token.address),
            pool_balance + lending_amount
        );
    }

    // borrower deposits first token and borrow second token
    let deposit_amount = 10_000_000_000;
    sut.pool
        .deposit(&borrower, &sut.reserves[0].token.address, &deposit_amount);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

    let borrower_stoken_balance = sut.reserves[0].s_token.balance(&borrower);
    let borrower_token_balance = sut.reserves[0].token.balance(&borrower);

    assert_eq!(borrower_stoken_balance, 10_000_000_000);
    assert_eq!(borrower_token_balance, 90000000000);
    assert_eq!(s_token_underlying_supply, 10_000_000_000);

    env.ledger().with_mut(|li| {
        li.timestamp = 30 * DAY;
    });

    // borrower borrows second token
    let borrowing_amount = 5_000_000_000;
    sut.pool
        .borrow(&borrower, &sut.reserves[1].token.address, &borrowing_amount);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

    let collat_coeff = FixedI128::from_inner(sut.pool.collat_coeff(&sut.reserves[1].token.address));
    std::println!("collat_coeff={:?}", collat_coeff.into_inner());

    let debt_coeff = FixedI128::from_inner(sut.pool.debt_coeff(&sut.reserves[1].token.address));
    std::println!("debt_coeff={:?}", debt_coeff.into_inner());

    let borrower_debt_amount = sut.reserves[1].debt_token.balance(&borrower);
    let borrower_token_amount = sut.reserves[1].token.balance(&borrower);
    let second_stoken_balance = sut.reserves[1].token.balance(&second_stoken_address);
    let treasury_balance = sut.reserves[1].token.balance(treasury_address);

    assert_eq!(borrower_debt_amount, 4991799920);
    assert_eq!(borrower_token_amount, 105000000000);
    assert_eq!(second_stoken_balance, 5000000000);
    assert_eq!(treasury_balance, 0);
    assert_eq!(s_token_underlying_supply, 5_000_000_000);

    // borrower partially repays second token
    let repayment_amount = 2_000_000_000;
    let repayment_amount_debt_token = debt_coeff.recip_mul_int(repayment_amount).unwrap();
    sut.pool
        .deposit(&borrower, &sut.reserves[1].token.address, &repayment_amount);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

    let expected_borrower_debt_amount = borrower_debt_amount - repayment_amount_debt_token;
    assert_eq!(expected_borrower_debt_amount, 2995079952_i128);

    let borrower_debt_amount = sut.reserves[1].debt_token.balance(&borrower);
    let borrower_token_amount = sut.reserves[1].token.balance(&borrower);
    let second_stoken_balance = sut.reserves[1].token.balance(&second_stoken_address);
    let treasury_balance = sut.reserves[1].token.balance(treasury_address);

    assert_eq!(borrower_debt_amount, expected_borrower_debt_amount);
    assert_eq!(borrower_token_amount, 103000000000);
    assert_eq!(second_stoken_balance, 6996556234);
    assert_eq!(treasury_balance, 3443766);
    assert_eq!(s_token_underlying_supply, 6_996_556_234);

    let debt_coeff = FixedI128::from_inner(sut.pool.debt_coeff(&sut.reserves[1].token.address));

    // borrower over-repays second token
    let over_repayment_amount = 7_000_000_000;
    let remaining_debt = debt_coeff.mul_int(borrower_debt_amount).unwrap();

    sut.pool.deposit(
        &borrower,
        &sut.reserves[1].token.address,
        &over_repayment_amount,
    );

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[1].s_token.address);

    let collat_coeff = FixedI128::from_inner(sut.pool.collat_coeff(&sut.reserves[1].token.address));
    let expected_deposit_amount = over_repayment_amount - remaining_debt;
    let expected_s_token_balance = collat_coeff.recip_mul_int(expected_deposit_amount).unwrap();
    assert_eq!(expected_s_token_balance, 4003820694);

    let borrower_debt_amount = sut.reserves[1].debt_token.balance(&borrower);
    let borrower_token_amount = sut.reserves[1].token.balance(&borrower);
    let second_stoken_balance = sut.reserves[1].token.balance(&second_stoken_address);
    let treasury_balance = sut.reserves[1].token.balance(treasury_address);
    let borrower_stoken_balance = sut.reserves[1].s_token.balance(&borrower);

    assert_eq!(borrower_debt_amount, 0);
    assert_eq!(borrower_token_amount, 96_000_000_000);
    assert_eq!(second_stoken_balance, 13990457389);
    assert_eq!(treasury_balance, 9542611);
    assert_eq!(borrower_stoken_balance, 4003820694);
    assert_eq!(s_token_underlying_supply, 13_990_457_389);
}

/// Fill lending pool with one lender and one borrower
/// Lender deposit all three assets.
/// Borrower deposit 0 asset and borrow 1 asset
fn fill_pool<'a, 'b>(env: &'b Env, sut: &'a Sut) -> (Address, Address, &'a ReserveConfig<'a>) {
    let initial_amount: i128 = 1_000_000_000;
    let lender = Address::random(&env);
    let borrower = Address::random(&env);
    let debt_token = sut.reserves[1].token.address.clone();

    for r in sut.reserves.iter() {
        r.token_admin.mint(&lender, &initial_amount);
        assert_eq!(r.token.balance(&lender), initial_amount);

        r.token_admin.mint(&borrower, &initial_amount);
        assert_eq!(r.token.balance(&borrower), initial_amount);
    }

    //lender deposit all tokens
    let deposit_amount = 100_000_000;
    for r in sut.reserves.iter() {
        let pool_balance = r.token.balance(&r.s_token.address);
        sut.pool.deposit(&lender, &r.token.address, &deposit_amount);
        assert_eq!(r.s_token.balance(&lender), deposit_amount);
        assert_eq!(
            r.token.balance(&r.s_token.address),
            pool_balance + deposit_amount
        );
    }

    //borrower deposit first token and borrow second token
    sut.pool
        .deposit(&borrower, &sut.reserves[0].token.address, &deposit_amount);
    assert_eq!(sut.reserves[0].s_token.balance(&borrower), deposit_amount);

    let borrow_amount = 40_000_000;
    sut.pool.borrow(&borrower, &debt_token, &borrow_amount);

    (lender, borrower, &sut.reserves[1])
}

/// Fill lending pool with two lenders and one borrower
fn fill_pool_two<'a, 'b>(
    env: &'b Env,
    sut: &'a Sut,
) -> (Address, Address, Address, &'a ReserveConfig<'a>) {
    let (lender_1, borrower, debt_token) = fill_pool(env, sut);

    let initial_amount: i128 = 1_000_000_000;
    let lender_2 = Address::random(env);

    for r in sut.reserves.iter() {
        r.token_admin.mint(&lender_2, &initial_amount);
        assert_eq!(r.token.balance(&lender_2), initial_amount);
    }

    //lender deposit all tokens
    let deposit_amount = 100_000_000;
    for r in sut.reserves.iter() {
        let pool_balance = r.token.balance(&r.s_token.address);
        sut.pool
            .deposit(&lender_2, &r.token.address, &deposit_amount);
        assert_eq!(r.s_token.balance(&lender_2), deposit_amount);
        assert_eq!(
            r.token.balance(&r.s_token.address),
            pool_balance + deposit_amount
        );
    }

    (lender_1, lender_2, borrower, debt_token)
}

#[test]
fn deposit_should_mint_s_token() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let (lender, _borrower, debt_config) = fill_pool(&env, &sut);
    let debt_token = &debt_config.token.address;
    // shift time to one day
    env.ledger().with_mut(|li| {
        li.timestamp = 24 * 60 * 60 // one day
    });

    let stoken_supply = debt_config.s_token.total_supply();
    let lender_stoken_balance_before = debt_config.s_token.balance(&lender);
    let deposit_amount = 10_000;
    sut.pool
        .deposit(&lender, &sut.reserves[1].token.address, &deposit_amount);

    let _reserve = sut.pool.get_reserve(&debt_token).unwrap();
    let collat_coeff = sut.pool.collat_coeff(&debt_token);
    let _debt_coeff = sut.pool.debt_coeff(&debt_token);

    let expected_stoken_amount = FixedI128::from_inner(collat_coeff)
        .recip_mul_int(deposit_amount)
        .unwrap();

    assert_eq!(
        debt_config.s_token.balance(&lender),
        lender_stoken_balance_before + expected_stoken_amount
    );
    assert_eq!(
        debt_config.s_token.total_supply(),
        stoken_supply + expected_stoken_amount
    );
    let collat_coeff_prev = sut.pool.collat_coeff(&debt_token);
    let debt_coeff_prev = sut.pool.debt_coeff(&debt_token);
    // shift time to one day
    env.ledger().with_mut(|li| {
        li.timestamp = 2 * 24 * 60 * 60 // one day
    });

    let collat_coeff = sut.pool.collat_coeff(&debt_token);
    let debt_coeff = sut.pool.debt_coeff(&debt_token);

    assert!(collat_coeff_prev < collat_coeff);
    assert!(debt_coeff_prev < debt_coeff);
}

#[test]
fn borrow_should_mint_debt_token() {
    let env = Env::default();
    env.mock_all_auths();

    //TODO: optimize gas

    let sut = init_pool(&env);

    env.budget().reset_unlimited();

    let (_lender, borrower, debt_config) = fill_pool(&env, &sut);
    let debt_token = &debt_config.token.address;

    // shift time to one day
    env.ledger().with_mut(|li| {
        li.timestamp = 24 * 60 * 60 // one day
    });

    let debttoken_supply = debt_config.debt_token.total_supply();
    let borrower_debt_token_balance_before = debt_config.debt_token.balance(&borrower);
    let borrow_amount = 10_000;
    sut.pool.borrow(&borrower, &debt_token, &borrow_amount);

    let reserve = sut.pool.get_reserve(&debt_token).unwrap();
    let expected_minted_debt_token = FixedI128::from_inner(reserve.borrower_accrued_rate)
        .recip_mul_int(borrow_amount)
        .unwrap();

    assert_eq!(
        debt_config.debt_token.balance(&borrower),
        borrower_debt_token_balance_before + expected_minted_debt_token
    );
    assert_eq!(
        debt_config.debt_token.balance(&borrower),
        debttoken_supply + expected_minted_debt_token
    )
}

#[test]
fn collateral_coeff_test() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    env.budget().reset_unlimited();

    let (_lender, borrower, debt_config) = fill_pool(&env, &sut);
    let initial_collat_coeff = sut.pool.collat_coeff(&debt_config.token.address);
    std::println!("initial_collat_coeff={}", initial_collat_coeff);

    env.ledger().with_mut(|l| {
        l.timestamp = 2 * DAY;
    });

    let borrow_amount = 50_000;
    sut.pool
        .borrow(&borrower, &debt_config.token.address, &borrow_amount);
    let reserve = sut.pool.get_reserve(&debt_config.token.address).unwrap();

    let collat_ar = FixedI128::from_inner(reserve.lender_accrued_rate);
    let s_token_supply = debt_config.s_token.total_supply();
    let balance = debt_config.token.balance(&debt_config.s_token.address);
    let debt_token_suply = debt_config.debt_token.total_supply();

    let expected_collat_coeff = FixedI128::from_rational(
        balance + collat_ar.mul_int(debt_token_suply).unwrap(),
        s_token_supply,
    )
    .unwrap()
    .into_inner();

    let collat_coeff = sut.pool.collat_coeff(&debt_config.token.address);
    assert_eq!(collat_coeff, expected_collat_coeff);

    // shift time to 8 days
    env.ledger().with_mut(|l| {
        l.timestamp = 10 * DAY;
    });

    let elapsed_time = 8 * DAY;
    let collat_ar = calc_next_accrued_rate(
        collat_ar,
        FixedI128::from_inner(reserve.lender_ir),
        elapsed_time,
    )
    .unwrap();
    let expected_collat_coeff = FixedI128::from_rational(
        balance + collat_ar.mul_int(debt_token_suply).unwrap(),
        s_token_supply,
    )
    .unwrap()
    .into_inner();

    let collat_coeff = sut.pool.collat_coeff(&debt_config.token.address);
    assert_eq!(collat_coeff, expected_collat_coeff);
    std::println!("collat_coeff={}", collat_coeff);
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn liquidity_cap_test() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    env.budget().reset_unlimited();

    let (lender, _borrower, debt_config) = fill_pool(&env, &sut);

    let token_one = 10_i128.pow(debt_config.token.decimals());
    let liq_bonus = 11000; //110%
    let liq_cap = 1_000_000 * 10_i128.pow(debt_config.token.decimals()); // 1M
    let discount = 6000; //60%
    let util_cap = 9000; //90%

    sut.pool.configure_as_collateral(
        &debt_config.token.address,
        &CollateralParamsInput {
            liq_bonus,
            liq_cap,
            discount,
            util_cap,
        },
    );

    //TODO: check error after soroban fix
    let deposit_amount = 1_000_000 * token_one;
    sut.pool
        .deposit(&lender, &debt_config.token.address, &deposit_amount);

    // assert_eq!(
    //     sut.pool
    //         .try_deposit(&lender, &debt_config.token.address, &deposit_amount)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::LiqCapExceeded
    // );
}

#[test]
fn repay_should_burn_debt_token() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let (_lender, borrower, debt_config) = fill_pool(&env, &sut);

    // shift time to one month
    env.ledger().with_mut(|li| {
        li.timestamp = 30 * 24 * 60 * 60 // one month
    });

    let debttoken_supply = debt_config.debt_token.total_supply();
    let borrower_debt_token_balance_before = debt_config.debt_token.balance(&borrower);
    let repay_amount = 100_000;
    sut.pool
        .deposit(&borrower, &debt_config.token.address, &repay_amount);

    let reserve = sut.pool.get_reserve(&debt_config.token.address).unwrap();
    let expected_burned_debt_token = FixedI128::from_inner(reserve.borrower_accrued_rate)
        .recip_mul_int(repay_amount)
        .unwrap();

    assert_eq!(
        debt_config.debt_token.balance(&borrower),
        borrower_debt_token_balance_before - expected_burned_debt_token
    );
    assert_eq!(
        debt_config.debt_token.total_supply(),
        debttoken_supply - expected_burned_debt_token
    );
}

#[test]
fn withdraw_should_burn_s_token() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    //TODO: optimize gas
    env.budget().reset_unlimited();

    let (lender, _borrower, debt_config) = fill_pool(&env, &sut);

    // shift time to one month
    env.ledger().with_mut(|li| {
        li.timestamp = 30 * 24 * 60 * 60 // one month
    });

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&debt_config.s_token.address);

    assert_eq!(s_token_underlying_supply, 60_000_000);

    let stoken_supply = debt_config.s_token.total_supply();
    let lender_stoken_balance_before = debt_config.s_token.balance(&lender);
    let withdraw_amount = 553_000;
    sut.pool.withdraw(
        &lender,
        &debt_config.token.address,
        &withdraw_amount,
        &lender,
    );

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&debt_config.s_token.address);

    let collat_coeff = FixedI128::from_inner(sut.pool.collat_coeff(&debt_config.token.address));
    let expected_burned_stoken = collat_coeff.recip_mul_int(withdraw_amount).unwrap();

    assert_eq!(
        debt_config.s_token.balance(&lender),
        lender_stoken_balance_before - expected_burned_stoken
    );
    assert_eq!(
        debt_config.s_token.total_supply(),
        stoken_supply - expected_burned_stoken
    );
    assert_eq!(s_token_underlying_supply, 59_447_000);
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn test_withdraw_bad_position() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    env.budget().reset_unlimited();

    let collateral = &sut.reserves[0].token;
    let collateral_admin = &sut.reserves[0].token_admin;

    let debt = &sut.reserves[1].token;
    let debt_admin = &sut.reserves[1].token_admin;

    let user = Address::random(&env);
    let lender = Address::random(&env);
    let deposit = 1_000_000_000;
    collateral_admin.mint(&user, &1_000_000_000);
    sut.pool.deposit(&user, &collateral.address, &deposit);
    let discount = sut
        .pool
        .get_reserve(&collateral.address)
        .expect("Reserve")
        .configuration
        .discount;
    let debt_amount = FixedI128::from_percentage(discount)
        .unwrap()
        .mul_int(deposit)
        .unwrap();
    debt_admin.mint(&lender, &deposit);
    sut.pool.deposit(&lender, &debt.address, &deposit);

    sut.pool.borrow(&user, &debt.address, &(debt_amount - 1));

    sut.pool
        .withdraw(&user, &collateral.address, &(deposit / 2), &user);

    //TODO: check error after soroban fix
    // assert_eq!(
    //     sut.pool
    //         .try_withdraw(&user, &collateral.address, &(deposit / 2), &user)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::BadPosition
    // );
}

#[test]
fn stoken_balance_not_changed_when_direct_transfer_to_underlying_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let lender = Address::random(&env);

    env.budget().reset_unlimited();

    sut.reserves[0].token_admin.mint(&lender, &2_000_000_000);
    sut.pool
        .deposit(&lender, &sut.reserves[0].token.address, &1_000_000_000);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[0].s_token.address);

    assert_eq!(s_token_underlying_supply, 1_000_000_000);

    sut.reserves[0]
        .token
        .transfer(&lender, &sut.reserves[0].s_token.address, &1_000_000_000);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[0].s_token.address);

    assert_eq!(s_token_underlying_supply, 1_000_000_000);
}
