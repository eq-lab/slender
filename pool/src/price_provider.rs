use pool_interface::Error;
use price_feed_interface::PriceFeedClient;
use soroban_sdk::{Address, Env};

#[allow(dead_code)]
pub struct PriceProvider<'a> {
    feed: PriceFeedClient<'a>,
}

pub struct AssetPrice {
    pub price: i128,
    pub decimals: u32,
}

#[allow(dead_code)]
impl PriceProvider<'_> {
    pub fn new(env: &Env, feed_address: &Address) -> Self {
        let feed = PriceFeedClient::new(env, feed_address);
        Self { feed }
    }

    pub fn get_price(&self, asset: &Address) -> Result<AssetPrice, Error> {
        let last_price = self.feed.lastprice(asset).ok_or(Error::NoPriceForAsset)?;
        let decimals: u32 = self.feed.decimals();

        if last_price.price <= 0 {
            return Err(Error::InvalidAssetPrice);
        }

        Ok(AssetPrice {
            price: last_price.price,
            decimals,
        })
    }
}
