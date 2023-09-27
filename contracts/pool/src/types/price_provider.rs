use common::FixedI128;
use pool_interface::types::error::Error;
use pool_interface::types::reserve_configuration::ReserveConfiguration;
use price_feed_interface::PriceFeedClient;
use soroban_sdk::{Address, Env, Map};

use crate::storage::read_price_feed;

pub struct PriceProvider<'a> {
    prices: Map<Address, i128>,
    env: &'a Env,
}

impl<'a> PriceProvider<'a> {
    pub fn new(env: &'a Env) -> Self {
        Self {
            env,
            prices: Map::new(env),
        }
    }

    pub fn price(
        &mut self,
        asset: &Address,
        config: &ReserveConfiguration,
    ) -> Result<FixedI128, Error> {
        if config.is_base_asset {
            return 10i128
                .checked_pow(config.decimals)
                .map(FixedI128::from_inner)
                .ok_or(Error::AssetPriceMathError);
        }

        let price = self.prices.get(asset.clone());

        let price = match price {
            Some(price) => price,
            None => {
                let feed = read_price_feed(self.env, asset)?;
                let client = PriceFeedClient::new(self.env, &feed);
                let price_data = client.lastprice(asset).ok_or(Error::NoPriceForAsset)?;

                if price_data.price <= 0 {
                    return Err(Error::InvalidAssetPrice);
                }

                self.prices.set(asset.clone(), price_data.price);

                price_data.price
            }
        };

        FixedI128::from_rational(price, 10i128.pow(config.decimals))
            .ok_or(Error::AssetPriceMathError)
    }
}
