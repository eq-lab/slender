#![cfg(test)]
extern crate std;

use crate::*;
use common::FixedI128;
use debt_token_interface::DebtTokenClient;
use flash_loan_receiver_interface::FlashLoanReceiverClient;
use price_feed_interface::PriceFeedClient;
use s_token_interface::STokenClient;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::token::StellarAssetClient as TokenAdminClient;
use soroban_sdk::Env;
use soroban_sdk::IntoVal;

mod pool {
    soroban_sdk::contractimport!(file = "../../target/wasm32-unknown-unknown/release/pool.wasm");
}

mod flash_loan_receiver {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/flash_loan_receiver_mock.wasm"
    );
}

mod s_token {
    soroban_sdk::contractimport!(file = "../../target/wasm32-unknown-unknown/release/s_token.wasm");
}

mod debt_token {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/debt_token.wasm"
    );
}

mod price_feed {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/price_feed_mock.wasm"
    );
}

pub const DAY: u64 = 24 * 60 * 60;

pub(crate) fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let stellar_asset_contract = e.register_stellar_asset_contract(admin.clone());
    (
        TokenClient::new(e, &stellar_asset_contract),
        TokenAdminClient::new(e, &stellar_asset_contract),
    )
}

pub(crate) fn create_pool_contract<'a>(
    e: &Env,
    admin: &Address,
    use_wasm: bool,
) -> LendingPoolClient<'a> {
    let client = if use_wasm {
        LendingPoolClient::new(e, &e.register_contract_wasm(None, pool::WASM))
    } else {
        LendingPoolClient::new(e, &e.register_contract(None, LendingPool))
    };

    let treasury = Address::random(e);
    let flash_loan_fee = 5;

    client.initialize(
        &admin,
        &treasury,
        &flash_loan_fee,
        &IRParams {
            alpha: 143,
            initial_rate: 200,
            max_rate: 50_000,
            scaling_coeff: 9_000,
        },
    );
    client
}

pub(crate) fn create_s_token_contract<'a>(
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

pub(crate) fn create_debt_token_contract<'a>(
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

pub(crate) fn create_price_feed_contract<'a>(e: &Env) -> PriceFeedClient<'a> {
    PriceFeedClient::new(&e, &e.register_contract_wasm(None, price_feed::WASM))
}

pub(crate) fn create_flash_loan_receiver_contract<'a>(e: &Env) -> FlashLoanReceiverClient<'a> {
    FlashLoanReceiverClient::new(
        &e,
        &e.register_contract_wasm(None, flash_loan_receiver::WASM),
    )
}

pub(crate) fn init_pool<'a>(env: &Env, use_pool_wasm: bool) -> Sut<'a> {
    env.budget().reset_unlimited();

    let admin = Address::random(&env);
    let token_admin = Address::random(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, use_pool_wasm);
    let price_feed: PriceFeedClient<'_> = create_price_feed_contract(&env);
    let flash_loan_receiver: FlashLoanReceiverClient<'_> =
        create_flash_loan_receiver_contract(&env);

    let reserves: std::vec::Vec<ReserveConfig<'a>> = (0..3)
        .map(|_| {
            let (token, token_admin_client) = create_token_contract(&env, &token_admin);
            let s_token = create_s_token_contract(&env, &pool.address, &token.address);
            let debt_token = create_debt_token_contract(&env, &pool.address, &token.address);
            let decimals = s_token.decimals();
            assert!(pool.get_reserve(&s_token.address).is_none());

            pool.init_reserve(
                &token.address,
                // &(i == 2),
                &InitReserveInput {
                    s_token_address: s_token.address.clone(),
                    debt_token_address: debt_token.address.clone(),
                    // decimals: 9,
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
            price_feed.set_price(&token.address.clone(), &FixedI128::DENOMINATOR);

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

    Sut {
        pool,
        price_feed,
        flash_loan_receiver,
        pool_admin: admin,
        token_admin: token_admin,
        reserves,
    }
}

/// Fill lending pool with one lender and one borrower
/// Lender deposits all three assets.
/// Borrower deposits 1 asset and borrow 1 asset
pub(crate) fn fill_pool<'a, 'b>(
    env: &'b Env,
    sut: &'a Sut,
    with_borrowing: bool,
) -> (Address, Address, &'a ReserveConfig<'a>) {
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

    env.ledger().with_mut(|li| li.timestamp = DAY);

    //borrower deposit first token and borrow second token
    sut.pool
        .deposit(&borrower, &sut.reserves[0].token.address, &deposit_amount);
    assert_eq!(sut.reserves[0].s_token.balance(&borrower), deposit_amount);

    if with_borrowing {
        let borrow_amount = 40_000_000;
        sut.pool.borrow(&borrower, &debt_token, &borrow_amount);
    }

    (lender, borrower, &sut.reserves[1])
}

/// Fill lending pool with two lenders and one borrower
pub(crate) fn fill_pool_two<'a, 'b>(
    env: &'b Env,
    sut: &'a Sut,
) -> (Address, Address, Address, &'a ReserveConfig<'a>) {
    let (lender_1, borrower, debt_token) = fill_pool(env, sut, true);

    let initial_amount: i128 = 1_000_000_000;
    let lender_2 = Address::random(env);

    for r in sut.reserves.iter() {
        r.token_admin.mint(&lender_2, &initial_amount);
        assert_eq!(r.token.balance(&lender_2), initial_amount);
    }

    env.ledger().with_mut(|li| li.timestamp = 2 * DAY);

    //lender deposit all tokens
    let deposit_amount = 100_000_000;
    for r in sut.reserves.iter() {
        let pool_balance = r.token.balance(&r.s_token.address);
        sut.pool
            .deposit(&lender_2, &r.token.address, &deposit_amount);
        assert_eq!(
            r.token.balance(&r.s_token.address),
            pool_balance + deposit_amount
        );
    }

    (lender_1, lender_2, borrower, debt_token)
}

/// Fill lending pool with lender, borrower, and liquidator
/// Borrower's position is ready for liquidation
pub(crate) fn fill_pool_three<'a, 'b>(
    env: &'b Env,
    sut: &'a Sut,
) -> (Address, Address, Address, &'a ReserveConfig<'a>) {
    let (lender, borrower, debt_config) = fill_pool(env, sut, false);
    let debt_token = debt_config.token.address.clone();

    let liquidator = Address::random(&env);

    env.ledger().with_mut(|li| li.timestamp = 2 * DAY);

    debt_config.token_admin.mint(&liquidator, &1_000_000_000);
    sut.pool.borrow(&borrower, &debt_token, &60_000_000);

    env.ledger().with_mut(|li| li.timestamp = 3 * DAY);

    (lender, borrower, liquidator, debt_config)
}

#[allow(dead_code)]
pub struct ReserveConfig<'a> {
    pub token: TokenClient<'a>,
    pub token_admin: TokenAdminClient<'a>,
    pub s_token: STokenClient<'a>,
    pub debt_token: DebtTokenClient<'a>,
}

#[allow(dead_code)]
pub struct Sut<'a> {
    pub pool: LendingPoolClient<'a>,
    pub price_feed: PriceFeedClient<'a>,
    pub flash_loan_receiver: FlashLoanReceiverClient<'a>,
    pub pool_admin: Address,
    pub token_admin: Address,
    pub reserves: std::vec::Vec<ReserveConfig<'a>>,
}

impl<'a> Sut<'a> {
    pub fn token(&self) -> &TokenClient<'a> {
        &self.reserves[0].token
    }

    pub fn token_admin(&self) -> &TokenAdminClient<'a> {
        &self.reserves[0].token_admin
    }

    pub fn debt_token(&self) -> &DebtTokenClient<'a> {
        &self.reserves[0].debt_token
    }

    pub fn s_token(&self) -> &STokenClient<'a> {
        &self.reserves[0].s_token
    }
}
