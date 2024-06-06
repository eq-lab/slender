#![cfg(test)]
extern crate std;

use crate::SToken;

use debt_token_interface::DebtTokenClient;
use s_token_interface::STokenClient;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation};
use soroban_sdk::token::{Client as TokenClient, StellarAssetClient as TokenAdminClient};
use soroban_sdk::{symbol_short, vec, Address, Env, IntoVal, Symbol};

use self::pool::{
    CollateralParamsInput, OracleAsset, PoolConfig, PriceFeed, PriceFeedConfigInput,
    ReserveType, TimestampPrecision,
};

mod pool {
    soroban_sdk::contractimport!(file = "../../target/wasm32-unknown-unknown/release/pool.wasm");
}

mod debt_token {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/debt_token.wasm"
    );
}

mod oracle {
    soroban_sdk::contractimport!(
        file = "../../target/wasm32-unknown-unknown/release/price_feed_mock.wasm"
    );
}

fn create_token<'a>(
    e: &Env,
) -> (
    STokenClient<'a>,
    DebtTokenClient<'a>,
    pool::Client<'a>,
    TokenClient,
    TokenAdminClient,
) {
    let pool_admin = Address::generate(e);

    let pool = pool::Client::new(e, &e.register_contract_wasm(None, pool::WASM));
    let s_token = STokenClient::new(e, &e.register_contract(None, SToken {}));
    let stellar_asset = &e.register_stellar_asset_contract(pool_admin.clone());

    let underlying_asset = TokenClient::new(e, stellar_asset);
    let underlying_asset_admin = TokenAdminClient::new(e, stellar_asset);

    let flash_loan_fee = 5;
    let initial_health = 2_500;
    let grace_period = 60 * 60 * 24;

    pool.initialize(
        &pool_admin,
        &PoolConfig {
            base_asset_address: underlying_asset.address.clone(),
            base_asset_decimals: 7,
            flash_loan_fee: flash_loan_fee,
            initial_health: initial_health,
            timestamp_window: 20,
            grace_period: grace_period,
            user_assets_limit: 4,
            min_collat_amount: 0,
            min_debt_amount: 0,
            liquidation_protocol_fee: 0,
            ir_alpha: 143,
            ir_initial_rate: 200,
            ir_max_rate: 50_000,
            ir_scaling_coeff: 9_000,
        },
    );

    e.budget().reset_default();
    let price_feed = oracle::Client::new(e, &e.register_contract_wasm(None, oracle::WASM));

    let feed_inputs = vec![
        &e,
        PriceFeedConfigInput {
            asset: underlying_asset.address.clone(),
            asset_decimals: 7,
            min_sanity_price_in_base: 5_000_000,
            max_sanity_price_in_base: 100_000_000,
            feeds: vec![
                &e,
                PriceFeed {
                    feed: price_feed.address.clone(),
                    feed_asset: OracleAsset::Stellar(underlying_asset.address.clone()),
                    feed_decimals: 14,
                    twap_records: 10,
                    min_timestamp_delta: 100,
                    timestamp_precision: TimestampPrecision::Sec,
                },
            ],
        },
    ];

    pool.set_price_feeds(&feed_inputs);

    s_token.initialize(
        &"name".into_val(e),
        &"symbol".into_val(e),
        &pool.address,
        &underlying_asset.address,
    );

    e.budget().reset_default();

    let debt_token: DebtTokenClient<'_> =
        DebtTokenClient::new(&e, &e.register_contract_wasm(None, debt_token::WASM));

    debt_token.initialize(
        &"DebtToken".into_val(e),
        &"DTOKEN".into_val(e),
        &pool.address,
        &underlying_asset.address,
    );

    (
        s_token,
        debt_token,
        pool,
        underlying_asset,
        underlying_asset_admin,
    )
}

#[test]
fn test() {
    let e = Env::default();
    e.mock_all_auths();

    let (s_token, debt_token, pool, underlying, underlying_admin) = create_token(&e);
    let init_reserve_input =
        ReserveType::Fungible(s_token.address.clone(), debt_token.address.clone());
    pool.init_reserve(&underlying.address, &init_reserve_input);

    e.budget().reset_default();

    {
        let underlying_decimals = underlying.decimals();
        let liq_cap = 100_000_000 * 10_i128.pow(underlying_decimals); // 100M
        let discount = 6000; //60%
        let util_cap = 9000; //90%
        let pen_order = 1;

        pool.configure_as_collateral(
            &underlying.address,
            &CollateralParamsInput {
                liq_cap,
                pen_order,
                discount,
                util_cap,
            },
        );
    }

    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let user3 = Address::generate(&e);

    underlying_admin.mint(&user1, &1000);

    underlying_admin.mint(&user2, &1000);

    s_token.mint(&user1, &1000);
    assert_eq!(
        e.auths(),
        [(
            pool.address.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    s_token.address.clone(),
                    symbol_short!("mint"),
                    (&user1, 1000_i128).into_val(&e),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
    assert_eq!(s_token.balance(&user1), 1000);
    assert_eq!(s_token.total_supply(), 1000);

    let min_expiration = e.ledger().sequence() + 1000;
    s_token.approve(&user2, &user3, &500, &min_expiration);
    assert_eq!(
        e.auths(),
        [(
            user2.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    s_token.address.clone(),
                    Symbol::new(&e, "approve"),
                    (&user2, &user3, 500_i128, min_expiration).into_val(&e),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
    assert_eq!(s_token.allowance(&user2, &user3), 500);

    s_token.transfer(&user1, &user2, &600);
    assert_eq!(
        e.auths(),
        [(
            user1.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    s_token.address.clone(),
                    symbol_short!("transfer"),
                    (&user1, &user2, 600_i128).into_val(&e),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
    assert_eq!(s_token.balance(&user1), 400);
    assert_eq!(s_token.balance(&user2), 600);

    s_token.transfer_from(&user3, &user2, &user1, &400);
    assert_eq!(
        e.auths(),
        [(
            user3.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    s_token.address.clone(),
                    Symbol::new(&e, "transfer_from"),
                    (&user3, &user2, &user1, 400_i128).into_val(&e),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
    assert_eq!(s_token.balance(&user1), 800);
    assert_eq!(s_token.balance(&user2), 200);

    s_token.transfer(&user1, &user3, &300);
    assert_eq!(s_token.balance(&user1), 500);
    assert_eq!(s_token.balance(&user3), 300);

    s_token.set_authorized(&user2, &false);
    assert_eq!(
        e.auths(),
        [(
            pool.address.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    s_token.address.clone(),
                    Symbol::new(&e, "set_authorized"),
                    (&user2, false).into_val(&e),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
    assert_eq!(s_token.authorized(&user2), false);

    s_token.set_authorized(&user3, &true);
    assert_eq!(s_token.authorized(&user3), true);

    s_token.clawback(&user3, &100);
    assert_eq!(
        e.auths(),
        [(
            pool.address.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    s_token.address.clone(),
                    symbol_short!("clawback"),
                    (&user3, 100_i128).into_val(&e),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
    assert_eq!(s_token.balance(&user3), 200);
    assert_eq!(s_token.total_supply(), 900);

    // Increase by 400, with an existing 100 = 500
    let min_expiration = e.ledger().sequence() + 1000;
    s_token.approve(&user2, &user3, &500, &min_expiration);
    assert_eq!(s_token.allowance(&user2, &user3), 500);

    s_token.approve(&user2, &user3, &0, &0);
    assert_eq!(
        e.auths(),
        [(
            user2.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    s_token.address.clone(),
                    Symbol::new(&e, "approve"),
                    (&user2, &user3, 0_i128, 0_u32).into_val(&e),
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
    assert_eq!(s_token.allowance(&user2, &user3), 0);
}

#[test]
#[should_panic(expected = "not implemented")]
fn test_burn() {
    let e = Env::default();
    e.mock_all_auths();

    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let (token, _, _pool, _, _) = create_token(&e);

    token.mint(&user1, &1000);
    assert_eq!(token.balance(&user1), 1000);
    assert_eq!(token.total_supply(), 1000);

    let min_expiration = e.ledger().sequence() + 1000;
    token.approve(&user1, &user2, &500, &min_expiration);
    assert_eq!(token.allowance(&user1, &user2), 500);

    token.burn_from(&user2, &user1, &500);
}

#[test]
#[should_panic(expected = "insufficient balance")]
fn transfer_insufficient_balance() {
    let e = Env::default();
    e.mock_all_auths();

    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let (token, _, _pool, _, _) = create_token(&e);

    token.mint(&user1, &1000);
    assert_eq!(token.balance(&user1), 1000);

    token.transfer(&user1, &user2, &1001);
}

#[test]
#[should_panic(expected = "can't receive when deauthorized")]
fn transfer_receive_deauthorized() {
    let e = Env::default();
    e.mock_all_auths();

    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let (token, _, _pool, _, _) = create_token(&e);

    token.mint(&user1, &1000);
    assert_eq!(token.balance(&user1), 1000);

    token.set_authorized(&user2, &false);
    token.transfer(&user1, &user2, &1);
}

#[test]
#[should_panic(expected = "can't spend when deauthorized")]
fn transfer_spend_deauthorized() {
    let e = Env::default();
    e.mock_all_auths();

    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let (token, _, _pool, _, _) = create_token(&e);

    token.mint(&user1, &1000);
    assert_eq!(token.balance(&user1), 1000);

    token.set_authorized(&user1, &false);
    token.transfer(&user1, &user2, &1);
}

#[test]
#[should_panic(expected = "insufficient allowance")]
fn transfer_from_insufficient_allowance() {
    let e = Env::default();
    e.mock_all_auths();

    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);
    let user3 = Address::generate(&e);
    let (token, _, _pool, _, _) = create_token(&e);

    token.mint(&user1, &1000);
    assert_eq!(token.balance(&user1), 1000);

    let min_expiration = e.ledger().sequence() + 1000;
    token.approve(&user1, &user3, &100, &min_expiration);
    assert_eq!(token.allowance(&user1, &user3), 100);

    token.transfer_from(&user3, &user1, &user2, &101);
}

#[test]
#[should_panic(expected = "s-token: already initialized")]
fn initialize_already_initialized() {
    let e = Env::default();
    e.mock_all_auths();
    let (token, _, _pool, _, _) = create_token(&e);

    let pool = Address::generate(&e);
    let underlying_asset = Address::generate(&e);

    token.initialize(
        &"name".into_val(&e),
        &"symbol".into_val(&e),
        &pool,
        &underlying_asset,
    );
}
