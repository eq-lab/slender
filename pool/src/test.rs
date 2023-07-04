use crate::*;
use s_token_interface::STokenClient;
use soroban_sdk::testutils::{Address as _, Events, MockAuth, MockAuthInvoke};
use soroban_sdk::{token, vec, IntoVal, Symbol};
use token::Client as TokenClient;

extern crate std;

mod s_token {
    soroban_sdk::contractimport!(file = "../target/wasm32-unknown-unknown/release/s_token.wasm");
}

fn create_token_contract<'a>(e: &Env, admin: &Address) -> TokenClient<'a> {
    TokenClient::new(e, &e.register_stellar_asset_contract(admin.clone()))
}

fn create_pool_contract<'a>(e: &Env, admin: &Address) -> LendingPoolClient<'a> {
    let client = LendingPoolClient::new(e, &e.register_contract(None, LendingPool));
    client.initialize(&admin);
    client
}

fn create_s_token_contract<'a>(
    e: &Env,
    pool: &Address,
    underlying_asset: &Address,
    treasury: &Address,
) -> STokenClient<'a> {
    let client = STokenClient::new(&e, &e.register_contract_wasm(None, s_token::WASM));

    client.initialize(
        &7,
        &"SToken".into_val(e),
        &"STOKEN".into_val(e),
        &pool,
        &treasury,
        &underlying_asset,
    );

    client
}

#[allow(dead_code)]
struct Sut<'a> {
    pool: LendingPoolClient<'a>,
    underlying_token: TokenClient<'a>,
    s_token: STokenClient<'a>,
    debt_token: TokenClient<'a>,
    pool_admin: Address,
    token_admin: Address,
}

fn init_pool<'a>(env: &Env) -> Sut<'a> {
    let admin = Address::random(&env);
    let token_admin = Address::random(&env);
    let treasury = Address::random(&env);

    let underlying_token = create_token_contract(&env, &token_admin);
    let debt_token = create_token_contract(&env, &token_admin);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin);
    let s_token =
        create_s_token_contract(&env, &pool.address, &underlying_token.address, &treasury);
    assert!(pool.get_reserve(&s_token.address).is_none());

    pool.init_reserve(
        &underlying_token.address,
        &InitReserveInput {
            s_token_address: s_token.address.clone(),
            debt_token_address: debt_token.address.clone(),
        },
    );

    Sut {
        pool,
        s_token,
        underlying_token,
        debt_token,
        pool_admin: admin,
        token_admin,
    }
}

#[test]
fn init_reserve() {
    let env = Env::default();

    let admin = Address::random(&env);
    let token_admin = Address::random(&env);
    let treasury = Address::random(&env);

    let underlying_token = create_token_contract(&env, &token_admin);
    let debt_token = create_token_contract(&env, &token_admin);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin);
    let s_token =
        create_s_token_contract(&env, &pool.address, &underlying_token.address, &treasury);
    assert!(pool.get_reserve(&underlying_token.address).is_none());

    let init_reserve_input = InitReserveInput {
        s_token_address: s_token.address.clone(),
        debt_token_address: debt_token.address.clone(),
    };

    assert_eq!(
        pool.mock_auths(&[MockAuth {
            address: &admin,
            nonce: 0,
            invoke: &MockAuthInvoke {
                contract: &pool.address,
                fn_name: "init_reserve",
                args: (&underlying_token.address, init_reserve_input.clone()).into_val(&env),
                sub_invokes: &[],
            }
        }])
        .init_reserve(&underlying_token.address, &init_reserve_input),
        ()
    );

    assert!(pool.get_reserve(&underlying_token.address).is_some());
}

#[test]
fn init_reserve_second_time() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let init_reserve_input = InitReserveInput {
        s_token_address: sut.s_token.address.clone(),
        debt_token_address: sut.debt_token.address.clone(),
    };

    assert_eq!(
        sut.pool
            .try_init_reserve(&sut.underlying_token.address, &init_reserve_input)
            .unwrap_err()
            .unwrap(),
        Error::ReserveAlreadyInitialized
    )
}

#[test]
fn init_reserve_when_pool_not_initialized() {
    let env = Env::default();

    let admin = Address::random(&env);
    let token_admin = Address::random(&env);
    let treasury = Address::random(&env);

    let underlying_token = create_token_contract(&env, &token_admin);
    let debt_token = create_token_contract(&env, &token_admin);

    let pool: LendingPoolClient<'_> =
        LendingPoolClient::new(&env, &env.register_contract(None, LendingPool));
    let s_token =
        create_s_token_contract(&env, &pool.address, &underlying_token.address, &treasury);
    assert!(pool.get_reserve(&underlying_token.address).is_none());

    let init_reserve_input = InitReserveInput {
        s_token_address: s_token.address.clone(),
        debt_token_address: debt_token.address.clone(),
    };

    assert_eq!(
        pool.mock_auths(&[MockAuth {
            address: &admin,
            nonce: 0,
            invoke: &MockAuthInvoke {
                contract: &pool.address,
                fn_name: "init_reserve",
                args: (&underlying_token.address, init_reserve_input.clone()).into_val(&env),
                sub_invokes: &[],
            }
        }])
        .try_init_reserve(&underlying_token.address, &init_reserve_input)
        .unwrap_err()
        .unwrap(),
        Error::Uninitialized
    );
}

#[test]
fn withdraw_base() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let user1 = Address::random(&env);
    let user2 = Address::random(&env);

    let initial_balance = 1_000_000_000;
    sut.underlying_token.mint(&user1, &1_000_000_000);
    assert_eq!(sut.underlying_token.balance(&user1), initial_balance);

    let deposit_amount = 10000;
    sut.pool
        .deposit(&user1, &sut.underlying_token.address, &deposit_amount);

    assert_eq!(sut.s_token.balance(&user1), deposit_amount);
    assert_eq!(
        sut.underlying_token.balance(&user1),
        initial_balance - deposit_amount
    );
    assert_eq!(
        sut.underlying_token.balance(&sut.s_token.address),
        deposit_amount
    );

    let amount_to_withdraw = 3500;
    sut.pool.withdraw(
        &user1,
        &sut.underlying_token.address,
        &amount_to_withdraw,
        &user2,
    );
    assert_eq!(sut.underlying_token.balance(&user2), amount_to_withdraw);
    assert_eq!(
        sut.s_token.balance(&user1),
        deposit_amount - amount_to_withdraw
    );
    assert_eq!(
        sut.underlying_token.balance(&sut.s_token.address),
        deposit_amount - amount_to_withdraw
    );

    let withdraw_event = env.events().all().pop_back_unchecked().unwrap();
    assert_eq!(
        vec![&env, withdraw_event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::short("withdraw"), &user1).into_val(&env),
                (&user2, &sut.underlying_token.address, amount_to_withdraw).into_val(&env)
            ),
        ]
    );

    sut.pool
        .withdraw(&user1, &sut.underlying_token.address, &i128::MAX, &user2);

    assert_eq!(sut.underlying_token.balance(&user2), deposit_amount);
    assert_eq!(sut.s_token.balance(&user1), 0);
    assert_eq!(sut.underlying_token.balance(&sut.s_token.address), 0);

    let withdraw_event = env.events().all().pop_back_unchecked().unwrap();
    assert_eq!(
        vec![&env, withdraw_event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::short("withdraw"), &user1).into_val(&env),
                (
                    &user2,
                    sut.underlying_token.address.clone(),
                    deposit_amount - amount_to_withdraw
                )
                    .into_val(&env)
            ),
        ]
    );

    let coll_disabled_event = env
        .events()
        .all()
        .get_unchecked(env.events().all().len() - 4)
        .unwrap();
    assert_eq!(
        vec![&env, coll_disabled_event],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (Symbol::new(&env, "reserve_used_as_coll_disabled"), &user1).into_val(&env),
                (sut.underlying_token.address.clone()).into_val(&env)
            ),
        ]
    );
}

#[test]
fn withdraw_interest_rate_less_than_one() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let user1 = Address::random(&env);
    let user2 = Address::random(&env);

    let initial_balance = 1_000_000_000;
    sut.underlying_token.mint(&user1, &1_000_000_000);
    assert_eq!(sut.underlying_token.balance(&user1), initial_balance);

    let liquidity_index = 500_000_000; //0.5
    sut.pool
        .set_liq_index(&sut.underlying_token.address, &liquidity_index);

    let deposit_amount = 1000;
    sut.pool
        .deposit(&user1, &sut.underlying_token.address, &deposit_amount);
    assert_eq!(sut.s_token.balance(&user1), 2000);
    assert_eq!(
        sut.underlying_token.balance(&user1),
        initial_balance - deposit_amount
    );
    assert_eq!(
        sut.underlying_token.balance(&sut.s_token.address),
        deposit_amount
    );

    let withdraw_amount = 500;
    sut.pool.withdraw(
        &user1,
        &sut.underlying_token.address,
        &withdraw_amount,
        &user2,
    );
    assert_eq!(sut.s_token.balance(&user1), 1000);
    assert_eq!(sut.underlying_token.balance(&sut.s_token.address), 500);
}

#[test]
fn withdraw_interest_rate_greater_than_one() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let user1 = Address::random(&env);
    let user2 = Address::random(&env);

    let initial_balance = 1_000_000_000;
    sut.underlying_token.mint(&user1, &1_000_000_000);
    assert_eq!(sut.underlying_token.balance(&user1), initial_balance);

    let liquidity_index = 1_200_000_000; //1.2
    sut.pool
        .set_liq_index(&sut.underlying_token.address, &liquidity_index);

    let deposit_amount = 1000;
    sut.pool
        .deposit(&user1, &sut.underlying_token.address, &deposit_amount);
    assert_eq!(sut.s_token.balance(&user1), 833);
    assert_eq!(
        sut.underlying_token.balance(&user1),
        initial_balance - deposit_amount
    );
    assert_eq!(
        sut.underlying_token.balance(&sut.s_token.address),
        deposit_amount
    );

    let withdraw_amount = 500;
    sut.pool.withdraw(
        &user1,
        &sut.underlying_token.address,
        &withdraw_amount,
        &user2,
    );
    assert_eq!(sut.s_token.balance(&user1), 417);
    assert_eq!(sut.underlying_token.balance(&sut.s_token.address), 500);
}

#[test]
fn withdraw_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let user1 = Address::random(&env);

    let withdraw_amount = 0;
    assert_eq!(
        sut.pool
            .try_withdraw(
                &user1,
                &sut.underlying_token.address,
                &withdraw_amount,
                &user1
            )
            .unwrap_err()
            .unwrap(),
        Error::InvalidAmount
    )
}

#[test]
fn withdraw_more_than_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let user1 = Address::random(&env);

    let initial_balance = 1_000_000_000;
    sut.underlying_token.mint(&user1, &1_000_000_000);
    assert_eq!(sut.underlying_token.balance(&user1), initial_balance);

    let deposit_amount = 1000;
    sut.pool
        .deposit(&user1, &sut.underlying_token.address, &deposit_amount);

    let withdraw_amount = 2000;
    assert_eq!(
        sut.pool
            .try_withdraw(
                &user1,
                &sut.underlying_token.address,
                &withdraw_amount,
                &user1
            )
            .unwrap_err()
            .unwrap(),
        Error::NotEnoughAvailableUserBalance
    )
}

#[test]
fn withdraw_unknown_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let user1 = Address::random(&env);

    let withdraw_amount = 1000;
    assert_eq!(
        sut.pool
            .try_withdraw(&user1, &sut.debt_token.address, &withdraw_amount, &user1)
            .unwrap_err()
            .unwrap(),
        Error::NoReserveExistForAsset
    )
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

    for i in 0..10 {
        let user = Address::random(&env);
        let initial_balance = 1_000_000_000;
        sut.underlying_token.mint(&user, &1_000_000_000);
        assert_eq!(sut.underlying_token.balance(&user), initial_balance);

        let deposit_amount = 1_000_0;
        let liq_index = common::RATE_DENOMINATOR + i * 100_000_000;
        assert_eq!(
            sut.pool
                .set_liq_index(&sut.underlying_token.address, &liq_index),
            ()
        );
        sut.pool
            .deposit(&user, &sut.underlying_token.address, &deposit_amount);

        assert_eq!(
            sut.s_token.balance(&user),
            deposit_amount * common::RATE_DENOMINATOR / liq_index
        );
        assert_eq!(
            sut.underlying_token.balance(&user),
            initial_balance - deposit_amount
        );

        let last = env.events().all().pop_back_unchecked().unwrap();
        assert_eq!(
            vec![&env, last],
            vec![
                &env,
                (
                    sut.pool.address.clone(),
                    (Symbol::short("deposit"), user).into_val(&env),
                    (sut.underlying_token.address.clone(), deposit_amount).into_val(&env)
                ),
            ]
        );

        env.budget().reset_default();
    }
}

#[test]
fn deposit_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let user1 = Address::random(&env);

    let deposit_amount = 0;
    assert_eq!(
        sut.pool
            .try_deposit(&user1, &sut.underlying_token.address, &deposit_amount,)
            .unwrap_err()
            .unwrap(),
        Error::InvalidAmount
    )
}

#[test]
fn deposit_non_active_reserve() {
    //TODO: implement when possible
}

#[test]
fn deposit_frozen_() {
    //TODO: implement when possible
}
