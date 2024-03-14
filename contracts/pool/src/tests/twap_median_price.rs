use pool_interface::types::oracle_asset::OracleAsset;
use pool_interface::types::price_feed::PriceFeed;
use pool_interface::types::price_feed_config_input::PriceFeedConfigInput;
use pool_interface::types::timestamp_precision::TimestampPrecision;
use price_feed_interface::types::asset::Asset;
use price_feed_interface::types::price_data::PriceData;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{symbol_short, vec, Address, Env};

use crate::tests::sut::init_pool;

use super::sut::create_price_feed_contract;

#[test]
#[should_panic(expected = "HostError: Error(Contract, #2)")]
fn should_fail_when_feed_is_missing_for_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let asset_1 = sut.reserves[1].token.address.clone();
    let asset_2 = Address::generate(&env).clone();

    let asset_1_feed_1 = create_price_feed_contract(&env);

    asset_1_feed_1.init(
        &Asset::Stellar(asset_1.clone()),
        &vec![
            &env,
            PriceData {
                price: 120_000_000_000_000_000,
                timestamp: 1704790200000,
            },
        ],
    );

    let price_feeds = vec![
        &env,
        PriceFeedConfigInput {
            asset: asset_1.clone(),
            asset_decimals: 9,
            feeds: vec![
                &env,
                // price feed with Stellar asset
                PriceFeed {
                    feed: asset_1_feed_1.address.clone(),
                    feed_asset: OracleAsset::Stellar(asset_1.clone()),
                    feed_decimals: 14,
                    twap_records: 10,
                    timestamp_precision: TimestampPrecision::Sec,
                },
            ],
        },
    ];

    sut.pool.set_price_feeds(&price_feeds);

    env.ledger().with_mut(|li| li.timestamp = 1704790800000); // delta = 600_000

    sut.pool.twap_median_price(&asset_2, &1_000_000_000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #104)")]
fn should_fail_when_price_is_missing_in_feed() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let asset_1 = sut.reserves[1].token.address.clone();

    let asset_1_feed_1 = create_price_feed_contract(&env);
    let asset_1_feed_2 = create_price_feed_contract(&env);

    asset_1_feed_1.init(
        &Asset::Stellar(asset_1.clone()),
        &vec![
            &env,
            PriceData {
                price: 120_000_000_000_000_000,
                timestamp: 1704790200000,
            },
        ],
    );

    // feed returns empty vec
    asset_1_feed_2.init(&Asset::Other(symbol_short!("XRP")), &vec![&env]);

    let price_feeds = vec![
        &env,
        PriceFeedConfigInput {
            asset: asset_1.clone(),
            asset_decimals: 9,
            feeds: vec![
                &env,
                // price feed with Stellar asset
                PriceFeed {
                    feed: asset_1_feed_1.address.clone(),
                    feed_asset: OracleAsset::Stellar(asset_1.clone()),
                    feed_decimals: 14,
                    twap_records: 10,
                    timestamp_precision: TimestampPrecision::Sec,
                },
                // price feed with Other asset
                PriceFeed {
                    feed: asset_1_feed_2.address.clone(),
                    feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                    feed_decimals: 10,
                    twap_records: 10,
                    timestamp_precision: TimestampPrecision::Sec,
                },
            ],
        },
    ];

    sut.pool.set_price_feeds(&price_feeds);

    env.ledger().with_mut(|li| li.timestamp = 1704790800000); // delta = 600_000

    sut.pool.twap_median_price(&asset_1, &1_000_000_000);
}

#[test]
fn should_return_twap_median_price() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let asset_1 = sut.reserves[1].token.address.clone();

    let asset_1_feed_1 = create_price_feed_contract(&env);
    let asset_1_feed_2 = create_price_feed_contract(&env);
    let asset_1_feed_3 = create_price_feed_contract(&env);

    asset_1_feed_1.init(
        &Asset::Stellar(asset_1.clone()),
        &vec![
            &env,
            PriceData {
                price: 120_000_000_000_000_000,
                timestamp: 1704790200, // delta = 300_000
            },
            PriceData {
                price: 100_000_000_000_000_000,
                timestamp: 1704789900, // delta = 300_000
            },
            PriceData {
                price: 90_000_000_000_000_000,
                timestamp: 1704789600, // delta = 300_000
            },
            PriceData {
                price: 100_000_000_000_000_000,
                timestamp: 1704789300, // delta = 0
            },
        ],
    );

    asset_1_feed_2.init(
        &Asset::Other(symbol_short!("XRP")),
        &vec![
            &env,
            PriceData {
                price: 10_000_000_000_000,
                timestamp: 1704790200000,
            },
        ],
    );

    let price_feeds = vec![
        &env,
        PriceFeedConfigInput {
            asset: asset_1.clone(),
            asset_decimals: 9,
            feeds: vec![
                &env,
                // price feed with Stellar asset
                PriceFeed {
                    feed: asset_1_feed_1.address.clone(),
                    feed_asset: OracleAsset::Stellar(asset_1.clone()),
                    feed_decimals: 14,
                    twap_records: 10,
                    timestamp_precision: TimestampPrecision::Sec,
                },
                // price feed with Other asset
                PriceFeed {
                    feed: asset_1_feed_2.address.clone(),
                    feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                    feed_decimals: 10,
                    twap_records: 10,
                    timestamp_precision: TimestampPrecision::Msec,
                },
            ],
        },
    ];

    sut.pool.set_price_feeds(&price_feeds);

    env.ledger().with_mut(|li| li.timestamp = 1704790800); // delta = 600_000
    extern crate std;

    let twap_median_price_1 = sut.pool.twap_median_price(&asset_1, &1_000_000_000);

    // median([1_060, 1_000]) = 1_030
    assert_eq!(twap_median_price_1, 10_300_000_000);

    asset_1_feed_3.init(
        &Asset::Other(symbol_short!("XRP")),
        &vec![
            &env,
            PriceData {
                price: 10_900_000_000_000,
                timestamp: 1704790200000,
            },
        ],
    );

    let price_feeds = vec![
        &env,
        PriceFeedConfigInput {
            asset: asset_1.clone(),
            asset_decimals: 9,
            feeds: vec![
                &env,
                // price feed with Stellar asset
                PriceFeed {
                    feed: asset_1_feed_1.address.clone(),
                    feed_asset: OracleAsset::Stellar(asset_1.clone()),
                    feed_decimals: 14,
                    twap_records: 10,
                    timestamp_precision: TimestampPrecision::Sec,
                },
                // price feed with Other asset
                PriceFeed {
                    feed: asset_1_feed_2.address.clone(),
                    feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                    feed_decimals: 10,
                    twap_records: 10,
                    timestamp_precision: TimestampPrecision::Msec,
                },
                PriceFeed {
                    feed: asset_1_feed_3.address.clone(),
                    feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                    feed_decimals: 10,
                    twap_records: 10,
                    timestamp_precision: TimestampPrecision::Msec,
                },
            ],
        },
    ];

    sut.pool.set_price_feeds(&price_feeds);

    let twap_median_price_2 = sut.pool.twap_median_price(&asset_1, &1_000_000_000);

    // median([1_060, 1_000, 1_090]) = 1_060
    assert_eq!(twap_median_price_2, 10_600_000_000);
}
