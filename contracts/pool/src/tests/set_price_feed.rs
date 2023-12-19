#![cfg(test)]
extern crate std;

use crate::tests::sut::{create_pool_contract, create_price_feed_contract};
use crate::*;
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

    assert!(pool.price_feed(&asset_1.clone()).is_none());
    assert!(pool.price_feed(&asset_2.clone()).is_none());

    let feed_inputs = Vec::from_array(
        &env,
        [
            PriceFeedInput {
                asset: asset_1.clone(),
                feed: price_feed.address.clone(),
                asset_decimals: 7,
                feed_decimals: 14,
            },
            PriceFeedInput {
                asset: asset_2.clone(),
                feed: price_feed.address.clone(),
                asset_decimals: 9,
                feed_decimals: 16,
            },
        ],
    );

    pool.set_price_feed(&feed_inputs);

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

    assert!(pool.price_feed(&asset_1.clone()).is_none());
    assert!(pool.price_feed(&asset_2.clone()).is_none());

    let feed_inputs = Vec::from_array(
        &env,
        [
            PriceFeedInput {
                asset: asset_1.clone(),
                feed: price_feed.address.clone(),
                asset_decimals: 7,
                feed_decimals: 14,
            },
            PriceFeedInput {
                asset: asset_2.clone(),
                feed: price_feed.address.clone(),
                asset_decimals: 9,
                feed_decimals: 16,
            },
        ],
    );

    pool.set_price_feed(&feed_inputs);

    let feed_1 = pool.price_feed(&asset_1).unwrap();
    let feed_2 = pool.price_feed(&asset_2).unwrap();

    assert_eq!(feed_1.feed, price_feed.address);
    assert_eq!(feed_1.feed_decimals, 14);
    assert_eq!(feed_1.asset_decimals, 7);

    assert_eq!(feed_2.feed, price_feed.address);
    assert_eq!(feed_2.feed_decimals, 16);
    assert_eq!(feed_2.asset_decimals, 9);
}
