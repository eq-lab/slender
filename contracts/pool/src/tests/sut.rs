#![cfg(test)]
extern crate std;

use crate::*;
use debt_token_interface::DebtTokenClient;
use flash_loan_receiver_interface::FlashLoanReceiverClient;
use pool_interface::types::oracle_asset::OracleAsset;
use pool_interface::types::price_feed::PriceFeed;
use pool_interface::types::timestamp_precision::TimestampPrecision;
use price_feed_interface::types::asset::Asset;
use price_feed_interface::types::price_data::PriceData;
use price_feed_interface::PriceFeedClient;
use s_token_interface::STokenClient;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::token::StellarAssetClient as TokenAdminClient;
use soroban_sdk::IntoVal;
use soroban_sdk::{vec, Env};

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

    let treasury = Address::generate(e);
    let flash_loan_fee = 5;
    let initial_health = 0;

    client.initialize(
        &admin,
        &treasury,
        &flash_loan_fee,
        &initial_health,
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

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, use_pool_wasm);
    let price_feed: PriceFeedClient<'_> = create_price_feed_contract(&env);
    let flash_loan_receiver: FlashLoanReceiverClient<'_> =
        create_flash_loan_receiver_contract(&env);

    let reserves: std::vec::Vec<ReserveConfig<'a>> = (0..4)
        .map(|i| {
            let (token, token_admin_client) = create_token_contract(&env, &token_admin);
            let decimals = (i == 0).then(|| 7).unwrap_or(9);

            let (s_token, debt_token) = if i == 3 {
                pool.init_reserve(&token.address, &ReserveType::RWA);
                (None, None)
            } else {
                let s_token = create_s_token_contract(&env, &pool.address, &token.address);
                assert!(pool.get_reserve(&s_token.address).is_none());
                let debt_token = create_debt_token_contract(&env, &pool.address, &token.address);
                pool.init_reserve(
                    &token.address,
                    &ReserveType::Fungible(s_token.address.clone(), debt_token.address.clone()),
                );
                pool.enable_borrowing_on_reserve(&token.address, &true);
                (Some(s_token), Some(debt_token))
            };

            if i == 0 {
                pool.set_base_asset(&token.address, &decimals)
            }

            let liquidity_cap = 100_000_000 * 10_i128.pow(decimals); // 100M
            let pen_order = i + 1;
            let util_cap = 9000; //90%
            let discount = 6000; //60%

            pool.configure_as_collateral(
                &token.address,
                &CollateralParamsInput {
                    liq_cap: liquidity_cap,
                    pen_order: pen_order,
                    util_cap,
                    discount,
                },
            );

            let reserve = pool.get_reserve(&token.address);
            assert_eq!(reserve.is_some(), true);

            let reserve_config = reserve.unwrap().configuration;
            assert_eq!(reserve_config.borrowing_enabled, i != 3); // borrowing disabled only for third asset (RWA)
            assert_eq!(reserve_config.liquidity_cap, liquidity_cap);
            assert_eq!(reserve_config.util_cap, util_cap);
            assert_eq!(reserve_config.discount, discount);

            match i {
                0 => price_feed.init(
                    &Asset::Stellar(token.address.clone()),
                    &vec![
                        &env,
                        PriceData {
                            price: 100_000_000_000_000,
                            timestamp: 1704790200000,
                        },
                    ],
                ),
                1 => price_feed.init(
                    &Asset::Stellar(token.address.clone()),
                    &vec![
                        &env,
                        PriceData {
                            price: 10_000_000_000_000_000,
                            timestamp: 1704790200000,
                        },
                    ],
                ),
                2 => price_feed.init(
                    &Asset::Stellar(token.address.clone()),
                    &vec![
                        &env,
                        PriceData {
                            price: 10_000_000_000_000_000,
                            timestamp: 1704790200000,
                        },
                    ],
                ),
                3 => price_feed.init(
                    &Asset::Stellar(token.address.clone()),
                    &vec![
                        &env,
                        PriceData {
                            price: 10_000_000_000_000_000,
                            timestamp: 1704790200000,
                        },
                    ],
                ),
                _ => panic!(),
            };

            ReserveConfig {
                token,
                token_admin: token_admin_client,
                s_token,
                debt_token,
            }
        })
        .collect();

    let feed_inputs = Vec::from_array(
        &env,
        [
            PriceFeedConfigInput {
                asset: reserves[0].token.address.clone(),
                asset_decimals: 7,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed.address.clone(),
                        feed_asset: OracleAsset::Stellar(reserves[0].token.address.clone()),
                        feed_decimals: 14,
                        twap_records: 10,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
            PriceFeedConfigInput {
                asset: reserves[1].token.address.clone(),
                asset_decimals: 9,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed.address.clone(),
                        feed_asset: OracleAsset::Stellar(reserves[1].token.address.clone()),
                        feed_decimals: 16,
                        twap_records: 10,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
            PriceFeedConfigInput {
                asset: reserves[2].token.address.clone(),
                asset_decimals: 9,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed.address.clone(),
                        feed_asset: OracleAsset::Stellar(reserves[2].token.address.clone()),
                        feed_decimals: 16,
                        twap_records: 10,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
            PriceFeedConfigInput {
                asset: reserves[3].token.address.clone(),
                asset_decimals: 9,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed.address.clone(),
                        feed_asset: OracleAsset::Stellar(reserves[3].token.address.clone()),
                        feed_decimals: 15,
                        twap_records: 10,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
        ],
    );

    pool.set_price_feeds(&feed_inputs);

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
    let lender = Address::generate(&env);
    let borrower = Address::generate(&env);
    let debt_token = sut.reserves[1].token.address.clone();

    for i in 0..3 {
        let amount = (i == 0).then(|| 10_000_000).unwrap_or(1_000_000_000);

        sut.reserves[i].token_admin.mint(&lender, &amount);
        sut.reserves[i].token_admin.mint(&borrower, &amount);

        assert_eq!(sut.reserves[i].token.balance(&lender), amount);
        assert_eq!(sut.reserves[i].token.balance(&borrower), amount);
    }

    //lender deposit all tokens
    for i in 0..3 {
        let amount = (i == 0).then(|| 1_000_000).unwrap_or(100_000_000);
        let stoken = sut.reserves[i].s_token().address.clone();
        let token = sut.reserves[i].token.address.clone();
        let pool_balance = sut.reserves[i].token.balance(&stoken);

        sut.pool.deposit(&lender, &token, &amount);

        assert_eq!(sut.reserves[i].s_token().balance(&lender), amount);
        assert_eq!(
            sut.reserves[i].token.balance(&stoken),
            pool_balance + amount
        );
    }

    env.ledger().with_mut(|li| li.timestamp = DAY);

    //borrower deposit first token and borrow second token
    sut.pool
        .deposit(&borrower, &sut.reserves[0].token.address, &1_000_000);
    assert_eq!(sut.reserves[0].s_token().balance(&borrower), 1_000_000);

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
    let lender_2 = Address::generate(env);

    for i in 0..3 {
        let amount = (i == 0).then(|| 10_000_000).unwrap_or(1_000_000_000);

        sut.reserves[i].token_admin.mint(&lender_2, &amount);
        assert_eq!(sut.reserves[i].token.balance(&lender_2), amount);
    }

    env.ledger().with_mut(|li| li.timestamp = 2 * DAY);

    //lender deposit all tokens
    for i in 0..3 {
        let amount = (i == 0).then(|| 1_000_000).unwrap_or(100_000_000);
        let stoken = sut.reserves[i].s_token().address.clone();
        let token = sut.reserves[i].token.address.clone();
        let pool_balance = sut.reserves[i].token.balance(&stoken);

        sut.pool.deposit(&lender_2, &token, &amount);
        assert_eq!(
            sut.reserves[i].token.balance(&stoken),
            pool_balance + amount
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

    let liquidator = Address::generate(&env);

    env.ledger().with_mut(|li| li.timestamp = 2 * DAY);

    debt_config.token_admin.mint(&liquidator, &1_000_000_000);
    sut.pool.borrow(&borrower, &debt_token, &60_000_000);

    env.ledger().with_mut(|li| li.timestamp = 3 * DAY);

    (lender, borrower, liquidator, debt_config)
}

#[cfg(feature = "budget")]
pub(crate) fn fill_pool_four<'a, 'b>(env: &'b Env, sut: &'a Sut) -> (Address, Address, Address) {
    let lender = Address::generate(&env);
    let borrower1 = Address::generate(&env);
    let borrower2 = Address::generate(&env);

    for i in 0..3 {
        let amount = (i == 0).then(|| 1_000_000_000).unwrap_or(100_000_000_000);

        sut.reserves[i].token_admin.mint(&lender, &amount);
        sut.reserves[i].token_admin.mint(&borrower1, &amount);
        sut.reserves[i].token_admin.mint(&borrower2, &amount);

        let amount = (i == 0).then(|| 100_000_000).unwrap_or(10_000_000_000);

        sut.pool
            .deposit(&lender, &sut.reserves[i].token.address, &amount);
    }

    env.ledger().with_mut(|li| li.timestamp = 1 * DAY);

    sut.pool
        .deposit(&borrower1, &sut.reserves[0].token.address, &100_000_000);
    sut.pool
        .deposit(&borrower1, &sut.reserves[1].token.address, &10_000_000_000);
    sut.pool
        .borrow(&borrower1, &sut.reserves[2].token.address, &6_000_000_000);

    sut.pool
        .deposit(&borrower2, &sut.reserves[2].token.address, &20_000_000_000);
    sut.pool
        .borrow(&borrower2, &sut.reserves[0].token.address, &60_000_000);
    sut.pool
        .borrow(&borrower2, &sut.reserves[1].token.address, &5_999_000_000);

    env.ledger().with_mut(|li| li.timestamp = 2 * DAY);

    (lender, borrower1, borrower2)
}

pub(crate) fn fill_pool_six<'a, 'b>(env: &'b Env, sut: &'a Sut) -> (Address, Address) {
    let lender = Address::generate(&env);
    let liquidator = Address::generate(&env);
    let borrower = Address::generate(&env);

    for i in 0..3 {
        let amount = (i == 0)
            .then(|| 10_000_000_000)
            .unwrap_or(1_000_000_000_000);

        sut.reserves[i].token_admin.mint(&lender, &amount);
        sut.reserves[i].token_admin.mint(&borrower, &amount);
        sut.reserves[i].token_admin.mint(&liquidator, &amount);

        assert_eq!(sut.reserves[i].token.balance(&lender), amount);
        assert_eq!(sut.reserves[i].token.balance(&borrower), amount);
        assert_eq!(sut.reserves[i].token.balance(&liquidator), amount);
    }

    //lender deposit all tokens
    for i in 0..3 {
        let amount = (i == 0)
            .then(|| 10_000_000_000)
            .unwrap_or(1_000_000_000_000);
        let stoken = sut.reserves[i].s_token().address.clone();
        let token = sut.reserves[i].token.address.clone();
        let pool_balance = sut.reserves[i].token.balance(&stoken);

        sut.pool.deposit(&lender, &token, &amount);

        assert_eq!(sut.reserves[i].s_token().balance(&lender), amount);
        assert_eq!(
            sut.reserves[i].token.balance(&stoken),
            pool_balance + amount
        );
    }

    (liquidator, borrower)
}

#[allow(dead_code)]
pub struct ReserveConfig<'a> {
    pub token: TokenClient<'a>,
    pub token_admin: TokenAdminClient<'a>,
    pub s_token: Option<STokenClient<'a>>,
    pub debt_token: Option<DebtTokenClient<'a>>,
}

impl<'a> ReserveConfig<'a> {
    pub fn s_token(&self) -> &STokenClient<'a> {
        self.s_token.as_ref().unwrap()
    }

    pub fn debt_token(&self) -> &DebtTokenClient<'a> {
        self.debt_token.as_ref().unwrap()
    }
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
        &self.reserves[0].debt_token()
    }

    pub fn s_token(&self) -> &STokenClient<'a> {
        &self.reserves[0].s_token()
    }

    pub fn rwa_config(&self) -> &ReserveConfig<'a> {
        &self.reserves[3]
    }
}
