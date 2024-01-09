#![cfg(test)]
extern crate std;

use crate::tests::sut::{create_pool_contract, create_price_feed_contract};
use crate::*;
use pool_interface::types::oracle_asset::OracleAsset;
use pool_interface::types::price_feed::PriceFeed;
use price_feed_interface::PriceFeedClient;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation};
use soroban_sdk::{vec, IntoVal, Symbol};

#[test]
fn should_require_admin() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let asset_1 = Address::generate(&env);
    let asset_2 = Address::generate(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, false);
    let price_feed: PriceFeedClient<'_> = create_price_feed_contract(&env);

    assert!(pool.price_feeds(&asset_1.clone()).is_none());
    assert!(pool.price_feeds(&asset_2.clone()).is_none());

    let feed_inputs = Vec::from_array(
        &env,
        [
            PriceFeedConfigInput {
                asset: asset_1.clone(),
                asset_decimals: 7,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed.address.clone(),
                        feed_asset: OracleAsset::Stellar(asset_1),
                        feed_decimals: 14,
                        twap_records: 10,
                    },
                ],
            },
            PriceFeedConfigInput {
                asset: asset_2.clone(),
                asset_decimals: 9,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed.address.clone(),
                        feed_asset: OracleAsset::Stellar(asset_2),
                        feed_decimals: 16,
                        twap_records: 10,
                    },
                ],
            },
        ],
    );

    pool.set_price_feeds(&feed_inputs);

    assert_eq!(
        env.auths(),
        [(
            admin.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    pool.address.clone(),
                    Symbol::new(&env, "set_price_feed"),
                    vec![&env, feed_inputs.into_val(&env)]
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

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, false);
    let price_feed: PriceFeedClient<'_> = create_price_feed_contract(&env);

    assert!(pool.price_feeds(&asset_1.clone()).is_none());
    assert!(pool.price_feeds(&asset_2.clone()).is_none());

    let feed_inputs = Vec::from_array(
        &env,
        [
            PriceFeedConfigInput {
                asset: asset_1.clone(),
                asset_decimals: 7,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed.address.clone(),
                        feed_asset: OracleAsset::Stellar(asset_1),
                        feed_decimals: 14,
                        twap_records: 10,
                    },
                ],
            },
            PriceFeedConfigInput {
                asset: asset_2.clone(),
                asset_decimals: 9,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed.address.clone(),
                        feed_asset: OracleAsset::Stellar(asset_2),
                        feed_decimals: 16,
                        twap_records: 10,
                    },
                ],
            },
        ],
    );

    pool.set_price_feeds(&feed_inputs);

    // let feed_1 = pool.price_feeds(&asset_1).unwrap();
    // let feed_2 = pool.price_feeds(&asset_2).unwrap();

    // TODO: add more assertions
    // assert_eq!(feed_1.feed, price_feed.address);
    // assert_eq!(feed_1.feed_decimals, 14);
    // assert_eq!(feed_1.asset_decimals, 7);

    // TODO: add more assertions
    // assert_eq!(feed_2.feed, price_feed.address);
    // assert_eq!(feed_2.feed_decimals, 16);
    // assert_eq!(feed_2.asset_decimals, 9);
}
