#![cfg(test)]
extern crate std;

use crate::tests::sut::{create_pool_contract, create_price_feed_contract};
use crate::*;
use pool_interface::types::oracle_asset::OracleAsset;
use pool_interface::types::price_feed::PriceFeed;
use pool_interface::types::timestamp_precision::TimestampPrecision;
use price_feed_interface::PriceFeedClient;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation};
use soroban_sdk::{symbol_short, vec, IntoVal, Symbol};

#[test]
fn should_require_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let asset_1 = Address::generate(&env);
    let asset_2 = Address::generate(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, false, &asset_1);
    let price_feed: PriceFeedClient<'_> = create_price_feed_contract(&env);

    assert!(pool.price_feeds(&asset_1.clone()).is_none());
    assert!(pool.price_feeds(&asset_2.clone()).is_none());

    let feed_inputs = Vec::from_array(
        &env,
        [
            PriceFeedConfigInput {
                asset: asset_1.clone(),
                asset_decimals: 7,
                min_sanity_price_in_base: 5_000_000,
                max_sanity_price_in_base: 100_000_000,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed.address.clone(),
                        feed_asset: OracleAsset::Stellar(asset_1),
                        feed_decimals: 14,
                        twap_records: 10,
                        min_timestamp_delta: 100,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
            PriceFeedConfigInput {
                asset: asset_2.clone(),
                asset_decimals: 9,
                min_sanity_price_in_base: 5_000_000,
                max_sanity_price_in_base: 100_000_000,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed.address.clone(),
                        feed_asset: OracleAsset::Stellar(asset_2),
                        feed_decimals: 16,
                        twap_records: 10,
                        min_timestamp_delta: 100,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
        ],
    );

    let set_price_feed_owner = Address::generate(&env);
    pool.grant_permission(&admin, &set_price_feed_owner, &Permission::SetPriceFeeds);

    pool.set_price_feeds(&set_price_feed_owner, &feed_inputs);

    assert_eq!(
        env.auths(),
        [(
            set_price_feed_owner.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    pool.address.clone(),
                    Symbol::new(&env, "set_price_feeds"),
                    vec![
                        &env,
                        set_price_feed_owner.into_val(&env),
                        feed_inputs.into_val(&env)
                    ]
                )),
                sub_invocations: std::vec![]
            }
        )]
    );
}

#[test]
fn should_set_price_feed() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let asset_1 = Address::generate(&env);
    let asset_2 = Address::generate(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, false, &asset_1);
    let price_feed_1: PriceFeedClient<'_> = create_price_feed_contract(&env);
    let price_feed_2: PriceFeedClient<'_> = create_price_feed_contract(&env);

    assert!(pool.price_feeds(&asset_1.clone()).is_none());
    assert!(pool.price_feeds(&asset_2.clone()).is_none());

    let feed_inputs = Vec::from_array(
        &env,
        [
            PriceFeedConfigInput {
                asset: asset_1.clone(),
                asset_decimals: 7,
                min_sanity_price_in_base: 5_000_000,
                max_sanity_price_in_base: 100_000_000,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed_1.address.clone(),
                        feed_asset: OracleAsset::Stellar(asset_1.clone()),
                        feed_decimals: 14,
                        twap_records: 10,
                        min_timestamp_delta: 100,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
            PriceFeedConfigInput {
                asset: asset_2.clone(),
                asset_decimals: 9,
                min_sanity_price_in_base: 5_000_000,
                max_sanity_price_in_base: 100_000_000,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed_2.address.clone(),
                        feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                        feed_decimals: 16,
                        twap_records: 9,
                        min_timestamp_delta: 100,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
        ],
    );

    pool.set_price_feeds(&admin, &feed_inputs);

    let feed_1 = pool.price_feeds(&asset_1).unwrap();
    let feed_2 = pool.price_feeds(&asset_2).unwrap();

    assert_eq!(feed_1.asset_decimals, 7);
    assert_eq!(
        feed_1.feeds.get_unchecked(0).feed,
        price_feed_1.address.clone()
    );
    assert!(match feed_1.feeds.get_unchecked(0).feed_asset {
        OracleAsset::Stellar(asset) => asset == asset_1,
        _ => false,
    });
    assert_eq!(feed_1.feeds.get_unchecked(0).feed_decimals, 14);
    assert_eq!(feed_1.feeds.get_unchecked(0).twap_records, 10);

    assert_eq!(feed_2.asset_decimals, 9);
    assert_eq!(
        feed_2.feeds.get_unchecked(0).feed,
        price_feed_2.address.clone()
    );
    assert!(match feed_2.feeds.get_unchecked(0).feed_asset {
        OracleAsset::Other(asset) => asset == symbol_short!("XRP"),
        _ => false,
    });
    assert_eq!(feed_2.feeds.get_unchecked(0).feed_decimals, 16);
    assert_eq!(feed_2.feeds.get_unchecked(0).twap_records, 9);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_no_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let asset_1 = Address::generate(&env);
    let asset_2 = Address::generate(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, false, &asset_1);
    let price_feed_1: PriceFeedClient<'_> = create_price_feed_contract(&env);
    let price_feed_2: PriceFeedClient<'_> = create_price_feed_contract(&env);

    assert!(pool.price_feeds(&asset_1.clone()).is_none());
    assert!(pool.price_feeds(&asset_2.clone()).is_none());

    let feed_inputs = Vec::from_array(
        &env,
        [
            PriceFeedConfigInput {
                asset: asset_1.clone(),
                asset_decimals: 7,
                min_sanity_price_in_base: 5_000_000,
                max_sanity_price_in_base: 100_000_000,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed_1.address.clone(),
                        feed_asset: OracleAsset::Stellar(asset_1.clone()),
                        feed_decimals: 14,
                        twap_records: 10,
                        min_timestamp_delta: 100,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
            PriceFeedConfigInput {
                asset: asset_2.clone(),
                asset_decimals: 9,
                min_sanity_price_in_base: 5_000_000,
                max_sanity_price_in_base: 100_000_000,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed_2.address.clone(),
                        feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                        feed_decimals: 16,
                        twap_records: 9,
                        min_timestamp_delta: 100,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
        ],
    );

    let perm = Address::generate(&env);
    assert!(pool
        .permissioned(&Permission::Permission)
        .binary_search(&admin)
        .is_ok());
    pool.grant_permission(&admin, &perm, &Permission::SetPriceFeeds);
    let no_perm = Address::generate(&env);
    let permissioned = pool.permissioned(&Permission::SetPriceFeeds);

    assert!(permissioned.binary_search(&no_perm).is_err());

    pool.set_price_feeds(&no_perm, &feed_inputs);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_has_another_permission() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let asset_1 = Address::generate(&env);
    let asset_2 = Address::generate(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, false, &asset_1);
    let price_feed_1: PriceFeedClient<'_> = create_price_feed_contract(&env);
    let price_feed_2: PriceFeedClient<'_> = create_price_feed_contract(&env);

    assert!(pool.price_feeds(&asset_1.clone()).is_none());
    assert!(pool.price_feeds(&asset_2.clone()).is_none());

    let feed_inputs = Vec::from_array(
        &env,
        [
            PriceFeedConfigInput {
                asset: asset_1.clone(),
                asset_decimals: 7,
                min_sanity_price_in_base: 5_000_000,
                max_sanity_price_in_base: 100_000_000,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed_1.address.clone(),
                        feed_asset: OracleAsset::Stellar(asset_1.clone()),
                        feed_decimals: 14,
                        twap_records: 10,
                        min_timestamp_delta: 100,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
            PriceFeedConfigInput {
                asset: asset_2.clone(),
                asset_decimals: 9,
                min_sanity_price_in_base: 5_000_000,
                max_sanity_price_in_base: 100_000_000,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed_2.address.clone(),
                        feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                        feed_decimals: 16,
                        twap_records: 9,
                        min_timestamp_delta: 100,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
        ],
    );

    let perm = Address::generate(&env);
    assert!(pool
        .permissioned(&Permission::Permission)
        .binary_search(&admin)
        .is_ok());
    pool.grant_permission(&admin, &perm, &Permission::SetPriceFeeds);
    let another_perm = Address::generate(&env);
    pool.grant_permission(&admin, &perm, &Permission::ClaimProtocolFee);
    let permissioned = pool.permissioned(&Permission::SetPriceFeeds);

    assert!(permissioned.binary_search(&another_perm).is_err());

    pool.set_price_feeds(&another_perm, &feed_inputs);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn should_fail_if_permission_revoked() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let asset_1 = Address::generate(&env);
    let asset_2 = Address::generate(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, false, &asset_1);
    let price_feed_1: PriceFeedClient<'_> = create_price_feed_contract(&env);
    let price_feed_2: PriceFeedClient<'_> = create_price_feed_contract(&env);

    assert!(pool.price_feeds(&asset_1.clone()).is_none());
    assert!(pool.price_feeds(&asset_2.clone()).is_none());

    let feed_inputs = Vec::from_array(
        &env,
        [
            PriceFeedConfigInput {
                asset: asset_1.clone(),
                asset_decimals: 7,
                min_sanity_price_in_base: 5_000_000,
                max_sanity_price_in_base: 100_000_000,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed_1.address.clone(),
                        feed_asset: OracleAsset::Stellar(asset_1.clone()),
                        feed_decimals: 14,
                        twap_records: 10,
                        min_timestamp_delta: 100,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
            PriceFeedConfigInput {
                asset: asset_2.clone(),
                asset_decimals: 9,
                min_sanity_price_in_base: 5_000_000,
                max_sanity_price_in_base: 100_000_000,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed_2.address.clone(),
                        feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                        feed_decimals: 16,
                        twap_records: 9,
                        min_timestamp_delta: 100,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
        ],
    );

    let perm = Address::generate(&env);
    assert!(pool
        .permissioned(&Permission::Permission)
        .binary_search(&admin)
        .is_ok());
    pool.grant_permission(&admin, &perm, &Permission::SetPriceFeeds);
    let revoked_perm = Address::generate(&env);
    pool.grant_permission(&admin, &revoked_perm, &Permission::SetPriceFeeds);
    pool.revoke_permission(&admin, &revoked_perm, &Permission::SetPriceFeeds);
    let permissioned = pool.permissioned(&Permission::SetPriceFeeds);

    assert!(permissioned.binary_search(&revoked_perm).is_err());

    pool.set_price_feeds(&revoked_perm, &feed_inputs);
}
