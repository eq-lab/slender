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

    let admin = Address::random(&env);
    let asset_1 = Address::random(&env);
    let asset_2 = Address::random(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, false);
    let price_feed: PriceFeedClient<'_> = create_price_feed_contract(&env);
    let assets = vec![&env, asset_1.clone(), asset_2.clone()];

    assert!(pool.price_feed(&asset_1.clone()).is_none());
    assert!(pool.price_feed(&asset_2.clone()).is_none());

    pool.set_price_feed(&price_feed.address.clone(), &assets.clone());

    assert_eq!(
        env.auths(),
        [(
            admin.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    pool.address.clone(),
                    Symbol::new(&env, "set_price_feed"),
                    (&price_feed.address, assets.clone()).into_val(&env)
                )),
                sub_invocations: std::vec![]
            }
        )]
    );

    assert_eq!(pool.price_feed(&asset_1).unwrap(), price_feed.address);
    assert_eq!(pool.price_feed(&asset_2).unwrap(), price_feed.address);
}

#[test]
fn should_set_price_feed() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::random(&env);
    let asset_1 = Address::random(&env);
    let asset_2 = Address::random(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin, false);
    let price_feed: PriceFeedClient<'_> = create_price_feed_contract(&env);
    let assets = vec![&env, asset_1.clone(), asset_2.clone()];

    assert!(pool.price_feed(&asset_1.clone()).is_none());
    assert!(pool.price_feed(&asset_2.clone()).is_none());

    pool.set_price_feed(&price_feed.address.clone(), &assets.clone());

    assert_eq!(pool.price_feed(&asset_1).unwrap(), price_feed.address);
    assert_eq!(pool.price_feed(&asset_2).unwrap(), price_feed.address);
}
