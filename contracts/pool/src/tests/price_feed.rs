use crate::tests::sut::{create_pool_contract, create_price_feed_contract};
use crate::*;
use price_feed_interface::PriceFeedClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::vec;

#[test]
fn should_be_none_when_not_set() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::random(&env);
    let not_set_asset = Address::random(&env);
    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, false);

    let price_feed = pool.price_feed(&not_set_asset.clone());

    assert!(price_feed.is_none());
}

#[test]
fn should_return_price_feed() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::random(&env);
    let asset_1 = Address::random(&env);
    let asset_2 = Address::random(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, false);
    let price_feed: PriceFeedClient<'_> = create_price_feed_contract(&env);
    let assets = vec![&env, asset_1.clone(), asset_2.clone()];

    pool.set_price_feed(&price_feed.address.clone(), &assets.clone());

    let asset_1_price_feed = pool.price_feed(&asset_1).unwrap();
    let asset_2_price_feed = pool.price_feed(&asset_2).unwrap();

    assert_eq!(asset_1_price_feed, price_feed.address);
    assert_eq!(asset_2_price_feed, price_feed.address);
}
