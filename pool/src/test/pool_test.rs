use crate::*;
use common::FixedI128;
use debt_token_interface::DebtTokenClient;
use price_feed_interface::PriceFeedClient;
use s_token_interface::STokenClient;
use soroban_sdk::testutils::{Address as _, Events, MockAuth, MockAuthInvoke};
use soroban_sdk::{token::Client as TokenClient, vec, IntoVal, Symbol};

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

fn create_debt_token_contract<'a>(
    e: &Env,
    pool: &Address,
    underlying_asset: &Address,
) -> DebtTokenClient<'a> {
    let client: DebtTokenClient<'_> =
        DebtTokenClient::new(&e, &e.register_contract_wasm(None, debt_token::WASM));

    client.initialize(
        &7,
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

#[allow(dead_code)]
struct ReserveConfig<'a> {
    token: TokenClient<'a>,
    s_token: STokenClient<'a>,
    debt_token: DebtTokenClient<'a>,
}

#[allow(dead_code)]
struct Sut<'a> {
    pool: LendingPoolClient<'a>,
    price_feed: PriceFeedClient<'a>,
    pool_admin: Address,
    token_admin: Address,
    reserves: std::vec::Vec<ReserveConfig<'a>>,
}

impl<'a> Sut<'a> {
    fn token(&self) -> &TokenClient<'a> {
        &self.reserves[0].token
    }

    fn debt_token(&self) -> &DebtTokenClient<'a> {
        &self.reserves[0].debt_token
    }

    fn s_token(&self) -> &STokenClient<'a> {
        &self.reserves[0].s_token
    }
}

fn init_pool<'a>(env: &Env) -> Sut<'a> {
    let admin = Address::random(&env);
    let token_admin = Address::random(&env);
    let treasury = Address::random(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin);
    let price_feed: PriceFeedClient<'_> = create_price_feed_contract(&env);

    let reserves: std::vec::Vec<ReserveConfig<'a>> = (0..3)
        .map(|_i| {
            let token = create_token_contract(&env, &token_admin);
            let debt_token = create_debt_token_contract(&env, &pool.address, &token.address);
            let s_token = create_s_token_contract(&env, &pool.address, &token.address, &treasury);

            assert!(pool.get_reserve(&s_token.address).is_none());

            pool.init_reserve(
                &token.address,
                &InitReserveInput {
                    s_token_address: s_token.address.clone(),
                    debt_token_address: debt_token.address.clone(),
                    ir_params: IRParams {
                        alpha: 143,
                        initial_rate: 200,
                        max_rate: 50_000,
                        scaling_coeff: 9_000,
                    },
                },
            );

            let liq_bonus = 11000; //110%
            let liq_cap = 100_000_000 * FixedI128::DENOMINATOR; // 100M
            let discount = 6000; //60%

            pool.configure_as_collateral(
                &token.address,
                &CollateralParamsInput {
                    liq_bonus,
                    liq_cap,
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
            assert_eq!(reserve_config.discount, discount);

            pool.set_price_feed(
                &price_feed.address,
                &soroban_sdk::vec![env, token.address.clone()],
            );

            let pool_price_feed = pool.get_price_feed(&token.address);
            assert_eq!(pool_price_feed, Some(price_feed.address.clone()));

            ReserveConfig {
                token,
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
        ir_params: IRParams {
            alpha: 143,
            initial_rate: 200,
            max_rate: 50_000,
            scaling_coeff: 9_000,
        },
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
    assert_eq!(
        init_reserve_input.ir_params.alpha,
        reserve.ir_params.alpha
    );
    assert_eq!(
        init_reserve_input.ir_params.initial_rate,
        reserve.ir_params.initial_rate
    );
    assert_eq!(
        init_reserve_input.ir_params.max_rate,
        reserve.ir_params.max_rate
    );
    assert_eq!(
        init_reserve_input.ir_params.scaling_coeff,
        reserve.ir_params.scaling_coeff
    );
}

#[test]
fn init_reserve_second_time() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let init_reserve_input = InitReserveInput {
        s_token_address: sut.s_token().address.clone(),
        debt_token_address: sut.debt_token().address.clone(),
        ir_params: IRParams {
            alpha: 143,
            initial_rate: 200,
            max_rate: 50_000,
            scaling_coeff: 9_000,
        },
    };

    assert_eq!(
        sut.pool
            .try_init_reserve(&sut.token().address, &init_reserve_input)
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
        ir_params: IRParams {
            alpha: 143,
            initial_rate: 200,
            max_rate: 50_000,
            scaling_coeff: 9_000,
        },
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

    sut.pool
        .set_ir_params(&sut.token().address, &ir_params_input);

    let reserve = sut.pool.get_reserve(&sut.token().address).unwrap();

    assert_eq!(ir_params_input.alpha, reserve.ir_params.alpha);
    assert_eq!(
        ir_params_input.initial_rate,
        reserve.ir_params.initial_rate
    );
    assert_eq!(
        ir_params_input.max_rate,
        reserve.ir_params.max_rate
    );
    assert_eq!(
        ir_params_input.scaling_coeff,
        reserve.ir_params.scaling_coeff
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
    sut.token().mint(&user1, &1_000_000_000);
    assert_eq!(sut.token().balance(&user1), initial_balance);

    let deposit_amount = 10000;
    sut.pool
        .deposit(&user1, &sut.token().address, &deposit_amount);

    assert_eq!(sut.s_token().balance(&user1), deposit_amount);
    assert_eq!(
        sut.token().balance(&user1),
        initial_balance - deposit_amount
    );
    assert_eq!(sut.token().balance(&sut.s_token().address), deposit_amount);

    let amount_to_withdraw = 3500;
    sut.pool
        .withdraw(&user1, &sut.token().address, &amount_to_withdraw, &user2);
    assert_eq!(sut.token().balance(&user2), amount_to_withdraw);
    assert_eq!(
        sut.s_token().balance(&user1),
        deposit_amount - amount_to_withdraw
    );
    assert_eq!(
        sut.token().balance(&sut.s_token().address),
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
                (&user2, &sut.token().address, amount_to_withdraw).into_val(&env)
            ),
        ]
    );

    sut.pool
        .withdraw(&user1, &sut.token().address, &i128::MAX, &user2);

    assert_eq!(sut.token().balance(&user2), deposit_amount);
    assert_eq!(sut.s_token().balance(&user1), 0);
    assert_eq!(sut.token().balance(&sut.s_token().address), 0);

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
        .get_unchecked(env.events().all().len() - 4)
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
fn withdraw_interest_rate_less_than_one() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let token = &sut.reserves[0].token;
    let s_token = &sut.reserves[0].s_token;

    let user1 = Address::random(&env);
    let user2 = Address::random(&env);

    let initial_balance = 1_000_000_000;
    token.mint(&user1, &1_000_000_000);
    assert_eq!(token.balance(&user1), initial_balance);

    let collat_accrued_rate: Option<i128> = Some(500_000_000); //0.5
    sut.pool
        .set_accrued_rates(&token.address, &collat_accrued_rate, &None);

    let deposit_amount = 1000;
    sut.pool.deposit(&user1, &token.address, &deposit_amount);
    assert_eq!(s_token.balance(&user1), 2000);
    assert_eq!(token.balance(&user1), initial_balance - deposit_amount);
    assert_eq!(token.balance(&s_token.address), deposit_amount);

    let withdraw_amount = 500;
    sut.pool
        .withdraw(&user1, &token.address, &withdraw_amount, &user2);
    assert_eq!(s_token.balance(&user1), 1000);
    assert_eq!(token.balance(&s_token.address), 500);
}

#[test]
fn withdraw_interest_rate_greater_than_one() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let user1 = Address::random(&env);
    let user2 = Address::random(&env);
    let token = &sut.reserves[0].token;
    let s_token = &sut.reserves[0].s_token;

    let initial_balance = 1_000_000_000;
    token.mint(&user1, &1_000_000_000);
    assert_eq!(token.balance(&user1), initial_balance);

    let collat_accrued_rate: Option<i128> = Some(1_200_000_000); //0.5
    sut.pool
        .set_accrued_rates(&token.address, &collat_accrued_rate, &None);

    let deposit_amount = 1000;
    sut.pool.deposit(&user1, &token.address, &deposit_amount);
    assert_eq!(s_token.balance(&user1), 833);
    assert_eq!(token.balance(&user1), initial_balance - deposit_amount);
    assert_eq!(token.balance(&s_token.address), deposit_amount);

    let withdraw_amount = 500;
    sut.pool
        .withdraw(&user1, &token.address, &withdraw_amount, &user2);
    assert_eq!(s_token.balance(&user1), 417);
    assert_eq!(token.balance(&s_token.address), 500);
}

#[test]
fn withdraw_zero_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let token = &sut.reserves[0].token;

    let user1 = Address::random(&env);

    let withdraw_amount = 0;
    assert_eq!(
        sut.pool
            .try_withdraw(&user1, &token.address, &withdraw_amount, &user1)
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
    let token = &sut.reserves[0].token;

    let user1 = Address::random(&env);

    let initial_balance = 1_000_000_000;
    token.mint(&user1, &1_000_000_000);
    assert_eq!(token.balance(&user1), initial_balance);

    let deposit_amount = 1000;
    sut.pool.deposit(&user1, &token.address, &deposit_amount);

    let withdraw_amount = 2000;
    assert_eq!(
        sut.pool
            .try_withdraw(&user1, &token.address, &withdraw_amount, &user1)
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
    let unknown_asset = &sut.reserves[0].debt_token.address;

    let withdraw_amount = 1000;
    assert_eq!(
        sut.pool
            .try_withdraw(&user1, unknown_asset, &withdraw_amount, &user1)
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

    let token = &sut.reserves[0].token;
    let s_token = &sut.reserves[0].s_token;

    for i in 0..10 {
        let user = Address::random(&env);
        let initial_balance = 1_000_000_000;
        token.mint(&user, &1_000_000_000);
        assert_eq!(token.balance(&user), initial_balance);

        let deposit_amount = 1_000_0;
        let collat_accrued_rate = Some(FixedI128::ONE.into_inner() + i * 100_000_000);

        assert_eq!(
            sut.pool
                .set_accrued_rates(&token.address, &collat_accrued_rate, &None),
            ()
        );
        sut.pool.deposit(&user, &token.address, &deposit_amount);

        assert_eq!(
            s_token.balance(&user),
            deposit_amount * FixedI128::ONE.into_inner() / collat_accrued_rate.unwrap()
        );
        assert_eq!(token.balance(&user), initial_balance - deposit_amount);

        let last = env.events().all().pop_back_unchecked().unwrap();
        assert_eq!(
            vec![&env, last],
            vec![
                &env,
                (
                    sut.pool.address.clone(),
                    (Symbol::short("deposit"), user).into_val(&env),
                    (token.address.clone(), deposit_amount).into_val(&env)
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
            .try_deposit(&user1, &sut.reserves[0].token.address, &deposit_amount)
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

#[test]
fn borrow() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let initial_amount: i128 = 1_000_000_000;
    let lender = Address::random(&env);
    let borrower = Address::random(&env);

    for r in sut.reserves.iter() {
        r.token.mint(&lender, &initial_amount);
        assert_eq!(r.token.balance(&lender), initial_amount);

        r.token.mint(&borrower, &initial_amount);
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

    env.budget().reset_default();

    //borrower deposit first token and borrow second token
    sut.pool
        .deposit(&borrower, &sut.reserves[0].token.address, &deposit_amount);
    assert_eq!(sut.reserves[0].s_token.balance(&borrower), deposit_amount);

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
}

#[test]
fn borrow_user_confgig_not_exists() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let borrower = Address::random(&env);

    let borrow_amount = 0;
    assert_eq!(
        sut.pool
            .try_borrow(&borrower, &sut.reserves[0].token.address, &borrow_amount)
            .unwrap_err()
            .unwrap(),
        Error::UserConfigNotExists
    )
}

#[test]
fn borrow_collateral_is_zero() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let borrower = Address::random(&env);

    let initial_amount = 1_000_000_000;
    for r in sut.reserves.iter() {
        r.token.mint(&borrower, &initial_amount);
        assert_eq!(r.token.balance(&borrower), initial_amount);
    }

    let deposit_amount = 1000;
    sut.pool
        .deposit(&borrower, &sut.reserves[1].token.address, &deposit_amount);

    sut.pool.withdraw(
        &borrower,
        &sut.reserves[1].token.address,
        &deposit_amount,
        &borrower,
    );

    let borrow_amount = 100;
    assert_eq!(
        sut.pool
            .try_borrow(&borrower, &sut.reserves[0].token.address, &borrow_amount)
            .unwrap_err()
            .unwrap(),
        Error::CollateralNotCoverNewBorrow
    )
}

#[test]
fn borrow_no_active_reserve() {
    //TODO: implement
}

#[test]
fn borrow_reserve_is_frozen() {
    //TODO: implement
}

#[test]
fn borrow_collateral_not_cover_new_debt() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let borrower = Address::random(&env);

    let initial_amount = 1_000_000_000;
    for r in sut.reserves.iter() {
        r.token.mint(&borrower, &initial_amount);
        assert_eq!(r.token.balance(&borrower), initial_amount);
    }

    let deposit_amount = 500;
    sut.pool
        .deposit(&borrower, &sut.reserves[1].token.address, &deposit_amount);

    let borrow_amount = 1000;
    assert_eq!(
        sut.pool
            .try_borrow(&borrower, &sut.reserves[0].token.address, &borrow_amount)
            .unwrap_err()
            .unwrap(),
        Error::CollateralNotCoverNewBorrow
    )
}

#[test]
fn borrow_disabled_for_borrowing_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let initial_amount: i128 = 1_000_000_000;
    let lender = Address::random(&env);
    let borrower = Address::random(&env);

    for r in sut.reserves.iter() {
        r.token.mint(&lender, &initial_amount);
        assert_eq!(r.token.balance(&lender), initial_amount);

        r.token.mint(&borrower, &initial_amount);
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

    //borrower borrow second token
    let borrow_asset = sut.reserves[1].token.address.clone();
    let borrow_amount = 10_000;

    //disable second token for borrowing
    sut.pool.enable_borrowing_on_reserve(&borrow_asset, &false);
    let reserve = sut.pool.get_reserve(&borrow_asset);
    assert_eq!(reserve.unwrap().configuration.borrowing_enabled, false);
    assert_eq!(
        sut.pool
            .try_borrow(&borrower, &borrow_asset, &borrow_amount)
            .unwrap_err()
            .unwrap(),
        Error::BorrowingNotEnabled
    );
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

    assert!(pool.get_price_feed(&asset_1.clone()).is_none());
    assert!(pool.get_price_feed(&asset_2.clone()).is_none());

    assert_eq!(
        pool.mock_auths(&[MockAuth {
            address: &admin,
            nonce: 0,
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

    assert_eq!(pool.get_price_feed(&asset_1).unwrap(), price_feed.address);
    assert_eq!(pool.get_price_feed(&asset_2).unwrap(), price_feed.address);
}