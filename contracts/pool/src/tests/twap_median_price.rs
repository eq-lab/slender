use pool_interface::types::oracle_asset::OracleAsset;
use pool_interface::types::price_feed::PriceFeed;
use pool_interface::types::price_feed_config_input::PriceFeedConfigInput;
use pool_interface::types::timestamp_precision::TimestampPrecision;
use price_feed_interface::types::asset::Asset;
use price_feed_interface::types::price_data::PriceData;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{symbol_short, vec, Address, Env};

use crate::tests::sut::init_pool;

use super::sut::{create_price_feed_contract, set_time};

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
            min_sanity_price_in_base: 5_000_000,
            max_sanity_price_in_base: 50_000_000_000,
            feeds: vec![
                &env,
                // price feed with Stellar asset
                PriceFeed {
                    feed: asset_1_feed_1.address.clone(),
                    feed_asset: OracleAsset::Stellar(asset_1.clone()),
                    feed_decimals: 14,
                    twap_records: 10,
                    min_timestamp_delta: 100,
                    timestamp_precision: TimestampPrecision::Sec,
                },
            ],
        },
    ];

    sut.pool.set_price_feeds(&sut.pool_admin, &price_feeds);

    set_time(&env, &sut, 1704790800000, false); // delta = 600_000

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
            min_sanity_price_in_base: 5_000_000,
            max_sanity_price_in_base: 50_000_000_000,
            feeds: vec![
                &env,
                // price feed with Stellar asset
                PriceFeed {
                    feed: asset_1_feed_1.address.clone(),
                    feed_asset: OracleAsset::Stellar(asset_1.clone()),
                    feed_decimals: 14,
                    twap_records: 10,
                    min_timestamp_delta: 100,
                    timestamp_precision: TimestampPrecision::Sec,
                },
                // price feed with Other asset
                PriceFeed {
                    feed: asset_1_feed_2.address.clone(),
                    feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                    feed_decimals: 10,
                    twap_records: 10,
                    min_timestamp_delta: 100,
                    timestamp_precision: TimestampPrecision::Sec,
                },
            ],
        },
    ];

    sut.pool.set_price_feeds(&sut.pool_admin, &price_feeds);

    set_time(&env, &sut, 1704790800000, false); // delta = 600_000

    sut.pool.twap_median_price(&asset_1, &1_000_000_000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #104)")]
fn should_fail_when_all_price_feeds_throws_error() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let asset_1 = sut.reserves[1].token.address.clone();

    let asset_1_feed_1 = create_price_feed_contract(&env);
    let asset_1_feed_2 = create_price_feed_contract(&env);
    let asset_1_feed_3 = create_price_feed_contract(&env);

    asset_1_feed_1.init(&Asset::Stellar(asset_1.clone()), &vec![&env]);
    asset_1_feed_2.init(&Asset::Other(symbol_short!("XRP")), &vec![&env]);
    asset_1_feed_3.init(&Asset::Other(symbol_short!("XRP")), &vec![&env]);

    let price_feeds = vec![
        &env,
        PriceFeedConfigInput {
            asset: asset_1.clone(),
            asset_decimals: 9,
            min_sanity_price_in_base: 5_000_000,
            max_sanity_price_in_base: 50_000_000_000,
            feeds: vec![
                &env,
                PriceFeed {
                    feed: asset_1_feed_1.address.clone(),
                    feed_asset: OracleAsset::Stellar(asset_1.clone()),
                    feed_decimals: 14,
                    twap_records: 10,
                    min_timestamp_delta: 1_000_000,
                    timestamp_precision: TimestampPrecision::Sec,
                },
                PriceFeed {
                    feed: asset_1_feed_2.address.clone(),
                    feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                    feed_decimals: 10,
                    twap_records: 10,
                    min_timestamp_delta: 1_000_000,
                    timestamp_precision: TimestampPrecision::Msec,
                },
                PriceFeed {
                    feed: asset_1_feed_3.address.clone(),
                    feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                    feed_decimals: 10,
                    twap_records: 10,
                    min_timestamp_delta: 1_000_000,
                    timestamp_precision: TimestampPrecision::Msec,
                },
            ],
        },
    ];

    sut.pool.set_price_feeds(&sut.pool_admin, &price_feeds);

    sut.pool.twap_median_price(&asset_1, &1_000_000_000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #104)")]
fn should_fail_when_price_is_not_stale() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let asset_1 = sut.reserves[1].token.address.clone();

    let asset_1_feed_1 = create_price_feed_contract(&env);

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

    let price_feeds = vec![
        &env,
        PriceFeedConfigInput {
            asset: asset_1.clone(),
            asset_decimals: 9,
            min_sanity_price_in_base: 5_000_000,
            max_sanity_price_in_base: 50_000_000_000,
            feeds: vec![
                &env,
                PriceFeed {
                    feed: asset_1_feed_1.address.clone(),
                    feed_asset: OracleAsset::Stellar(asset_1.clone()),
                    feed_decimals: 10,
                    twap_records: 10,
                    min_timestamp_delta: 200,
                    timestamp_precision: TimestampPrecision::Sec,
                },
            ],
        },
    ];

    sut.pool.set_price_feeds(&sut.pool_admin, &price_feeds);

    set_time(&env, &sut, 1704790800, false); // delta = 600
    extern crate std;

    sut.pool.twap_median_price(&asset_1, &1_000_000_000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #106)")]
fn should_fail_when_price_is_below_min_sanity() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let asset_1 = sut.reserves[1].token.address.clone();

    let asset_1_feed_1 = create_price_feed_contract(&env);

    asset_1_feed_1.init(
        &Asset::Stellar(asset_1.clone()),
        &vec![
            &env,
            PriceData {
                price: 45_500_000_000,
                timestamp: 1704790200, // delta = 300_000
            },
            PriceData {
                price: 65_500_000_000,
                timestamp: 1704789900, // delta = 300_000
            },
            PriceData {
                price: 25_500_000_000,
                timestamp: 1704789600, // delta = 300_000
            },
            PriceData {
                price: 55_500_000_000,
                timestamp: 1704789300, // delta = 0
            },
        ],
    );

    let price_feeds = vec![
        &env,
        PriceFeedConfigInput {
            asset: asset_1.clone(),
            asset_decimals: 9,
            min_sanity_price_in_base: 50_000_000,
            max_sanity_price_in_base: 50_000_000_000,
            feeds: vec![
                &env,
                PriceFeed {
                    feed: asset_1_feed_1.address.clone(),
                    feed_asset: OracleAsset::Stellar(asset_1.clone()),
                    feed_decimals: 10,
                    twap_records: 10,
                    min_timestamp_delta: 1_000_000,
                    timestamp_precision: TimestampPrecision::Sec,
                },
            ],
        },
    ];

    sut.pool.set_price_feeds(&sut.pool_admin, &price_feeds);

    set_time(&env, &sut, 1704790800, false); // delta = 600
    extern crate std;

    sut.pool.twap_median_price(&asset_1, &1_000_000_000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #106)")]
fn should_fail_when_price_is_below_max_sanity() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let asset_1 = sut.reserves[1].token.address.clone();

    let asset_1_feed_1 = create_price_feed_contract(&env);

    asset_1_feed_1.init(
        &Asset::Stellar(asset_1.clone()),
        &vec![
            &env,
            PriceData {
                price: 1_000_000_000_000,
                timestamp: 1704790200, // delta = 300_000
            },
            PriceData {
                price: 990_000_000_000,
                timestamp: 1704789900, // delta = 300_000
            },
            PriceData {
                price: 1_200_000_000_000,
                timestamp: 1704789600, // delta = 300_000
            },
            PriceData {
                price: 1_800_000_000_000,
                timestamp: 1704789300, // delta = 0
            },
        ],
    );

    let price_feeds = vec![
        &env,
        PriceFeedConfigInput {
            asset: asset_1.clone(),
            asset_decimals: 9,
            min_sanity_price_in_base: 5_000_000,
            max_sanity_price_in_base: 1_000_000_000,
            feeds: vec![
                &env,
                PriceFeed {
                    feed: asset_1_feed_1.address.clone(),
                    feed_asset: OracleAsset::Stellar(asset_1.clone()),
                    feed_decimals: 10,
                    twap_records: 10,
                    min_timestamp_delta: 1_000_000,
                    timestamp_precision: TimestampPrecision::Sec,
                },
            ],
        },
    ];

    sut.pool.set_price_feeds(&sut.pool_admin, &price_feeds);

    set_time(&env, &sut, 1704790800, false); // delta = 600
    extern crate std;

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
            min_sanity_price_in_base: 5_000_000,
            max_sanity_price_in_base: 50_000_000_000,
            feeds: vec![
                &env,
                // price feed with Stellar asset
                PriceFeed {
                    feed: asset_1_feed_1.address.clone(),
                    feed_asset: OracleAsset::Stellar(asset_1.clone()),
                    feed_decimals: 14,
                    twap_records: 10,
                    min_timestamp_delta: 1_000_000,
                    timestamp_precision: TimestampPrecision::Sec,
                },
                // price feed with Other asset
                PriceFeed {
                    feed: asset_1_feed_2.address.clone(),
                    feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                    feed_decimals: 10,
                    twap_records: 10,
                    min_timestamp_delta: 1_000_000,
                    timestamp_precision: TimestampPrecision::Msec,
                },
            ],
        },
    ];

    sut.pool.set_price_feeds(&sut.pool_admin, &price_feeds);

    set_time(&env, &sut, 1704790800, false); // delta = 600_000
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
            min_sanity_price_in_base: 5_000_000,
            max_sanity_price_in_base: 50_000_000_000,
            feeds: vec![
                &env,
                // price feed with Stellar asset
                PriceFeed {
                    feed: asset_1_feed_1.address.clone(),
                    feed_asset: OracleAsset::Stellar(asset_1.clone()),
                    feed_decimals: 14,
                    twap_records: 10,
                    min_timestamp_delta: 1_000_000,
                    timestamp_precision: TimestampPrecision::Sec,
                },
                // price feed with Other asset
                PriceFeed {
                    feed: asset_1_feed_2.address.clone(),
                    feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                    feed_decimals: 10,
                    twap_records: 10,
                    min_timestamp_delta: 1_000_000,
                    timestamp_precision: TimestampPrecision::Msec,
                },
                PriceFeed {
                    feed: asset_1_feed_3.address.clone(),
                    feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                    feed_decimals: 10,
                    twap_records: 10,
                    min_timestamp_delta: 1_000_000,
                    timestamp_precision: TimestampPrecision::Msec,
                },
            ],
        },
    ];

    sut.pool.set_price_feeds(&sut.pool_admin, &price_feeds);

    let twap_median_price_2 = sut.pool.twap_median_price(&asset_1, &1_000_000_000);

    // median([1_060, 1_000, 1_090]) = 1_060
    assert_eq!(twap_median_price_2, 10_600_000_000);
}

#[test]
fn should_return_twap_median_price_when_unsorted_prices() {
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
                price: 100_000_000_000_000_000,
                timestamp: 1704789300,
            },
            PriceData {
                price: 100_000_000_000_000_000,
                timestamp: 1704789900,
            },
            PriceData {
                price: 120_000_000_000_000_000,
                timestamp: 1704790200,
            },
            PriceData {
                price: 90_000_000_000_000_000,
                timestamp: 1704789600,
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
            min_sanity_price_in_base: 5_000_000,
            max_sanity_price_in_base: 50_000_000_000,
            feeds: vec![
                &env,
                // price feed with Stellar asset
                PriceFeed {
                    feed: asset_1_feed_1.address.clone(),
                    feed_asset: OracleAsset::Stellar(asset_1.clone()),
                    feed_decimals: 14,
                    twap_records: 10,
                    min_timestamp_delta: 1_000_000,
                    timestamp_precision: TimestampPrecision::Sec,
                },
                // price feed with Other asset
                PriceFeed {
                    feed: asset_1_feed_2.address.clone(),
                    feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                    feed_decimals: 10,
                    twap_records: 10,
                    min_timestamp_delta: 1_000_000,
                    timestamp_precision: TimestampPrecision::Msec,
                },
            ],
        },
    ];

    sut.pool.set_price_feeds(&sut.pool_admin, &price_feeds);

    set_time(&env, &sut, 1704790800, false); // delta = 600_000
    extern crate std;

    let twap_median_price_1 = sut.pool.twap_median_price(&asset_1, &1_000_000_000);

    // median([1_060, 1_000]) = 1_030
    assert_eq!(twap_median_price_1, 10_300_000_000);
}

#[test]
fn should_use_backup_price_feed() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);

    let asset_1 = sut.reserves[1].token.address.clone();

    let asset_1_feed_1 = create_price_feed_contract(&env);
    let asset_1_feed_2 = create_price_feed_contract(&env);
    let asset_1_feed_3 = create_price_feed_contract(&env);

    asset_1_feed_3.init(
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

    asset_1_feed_2.init(&Asset::Other(symbol_short!("XRP")), &vec![&env]);
    asset_1_feed_2.init(
        &Asset::Other(symbol_short!("XRP")),
        &vec![
            &env,
            PriceData {
                price: 10_000_000_000_000,
                timestamp: 1704789700000, // not stale price
            },
        ],
    );

    let price_feeds = vec![
        &env,
        PriceFeedConfigInput {
            asset: asset_1.clone(),
            asset_decimals: 9,
            min_sanity_price_in_base: 5_000_000,
            max_sanity_price_in_base: 50_000_000_000,
            feeds: vec![
                &env,
                PriceFeed {
                    feed: asset_1_feed_1.address.clone(),
                    feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                    feed_decimals: 10,
                    twap_records: 10,
                    min_timestamp_delta: 600_000,
                    timestamp_precision: TimestampPrecision::Msec,
                },
                PriceFeed {
                    feed: asset_1_feed_2.address.clone(),
                    feed_asset: OracleAsset::Other(symbol_short!("XRP")),
                    feed_decimals: 10,
                    twap_records: 10,
                    min_timestamp_delta: 600_000,
                    timestamp_precision: TimestampPrecision::Msec,
                },
                // backup price oracle
                PriceFeed {
                    feed: asset_1_feed_3.address.clone(),
                    feed_asset: OracleAsset::Stellar(asset_1.clone()),
                    feed_decimals: 14,
                    twap_records: 10,
                    min_timestamp_delta: 600_000,
                    timestamp_precision: TimestampPrecision::Sec,
                },
            ],
        },
    ];

    sut.pool.set_price_feeds(&sut.pool_admin, &price_feeds);

    set_time(&env, &sut, 1704790800, false); // delta = 600_000
    extern crate std;

    let twap_median_price_1 = sut.pool.twap_median_price(&asset_1, &1_000_000_000);

    // median([1_060]) = 1_030
    assert_eq!(twap_median_price_1, 10_600_000_000);
}
