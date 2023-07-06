use soroban_sdk::{Env, Address};
use price_feed_interface::PriceFeedClient;

#[allow(dead_code)]
pub struct PriceProvider<'a> {
    feed: PriceFeedClient<'a>,
}

#[allow(dead_code)]
impl PriceProvider<'_> {
    pub fn new(env: &Env, feed_address: Address) -> Self {
        let feed = price_feed_interface::PriceFeedClient::new(&env, &feed_address);
        Self { feed }
    }

    pub fn get_price(&self, asset: Address) -> Option<i128> {
        let price_date = self.feed.lastprice(&asset);
        Some(price_date?.price)
    }
}
