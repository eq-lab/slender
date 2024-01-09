use common::FixedI128;
use pool_interface::types::base_asset_config::BaseAssetConfig;
use pool_interface::types::error::Error;
use pool_interface::types::price_feed::PriceFeed;
use pool_interface::types::price_feed_config::PriceFeedConfig;
use price_feed_interface::PriceFeedClient;
use soroban_sdk::{Address, Env, Map};

use crate::storage::{read_base_asset, read_price_feeds};

pub struct PriceProvider<'a> {
    env: &'a Env,
    base_asset: Option<BaseAssetConfig>,
    configs: Map<Address, PriceFeedConfig>,
    prices: Map<Address, i128>,
}

impl<'a> PriceProvider<'a> {
    pub fn new(env: &'a Env) -> Result<Self, Error> {
        Ok(Self {
            env,
            base_asset: None,
            configs: Map::new(env),
            prices: Map::new(env),
        })
    }

    pub fn convert_to_base(&mut self, asset: &Address, amount: i128) -> Result<i128, Error> {
        let base_asset = self.base_asset()?;

        if base_asset.address == *asset {
            return Ok(amount);
        }

        let config = self.config(asset)?;
        let price = self.price(asset, &config)?;

        FixedI128::from_inner(price)
            .mul_int(amount)
            .and_then(|a| FixedI128::from_rational(a, 10i128.pow(config.asset_decimals)))
            .and_then(|a| a.to_precision(base_asset.decimals))
            .ok_or(Error::InvalidAssetPrice)
    }

    pub fn convert_from_base(&mut self, asset: &Address, amount: i128) -> Result<i128, Error> {
        let base_asset = self.base_asset()?;

        if base_asset.address == *asset {
            return Ok(amount);
        }

        let config = self.config(asset)?;
        let price = self.price(asset, &config)?;

        FixedI128::from_inner(price)
            .recip_mul_int(amount)
            .and_then(|a| FixedI128::from_rational(a, 10i128.pow(base_asset.decimals)))
            .and_then(|a| a.to_precision(config.asset_decimals))
            .ok_or(Error::InvalidAssetPrice)
    }

    fn base_asset(&mut self) -> Result<BaseAssetConfig, Error> {
        match &self.base_asset {
            Some(config) => Ok(config.clone()),
            None => {
                let base_asset = read_base_asset(self.env)?;
                self.base_asset = Some(base_asset.clone());

                Ok(base_asset)
            }
        }
    }

    fn config(&mut self, asset: &Address) -> Result<PriceFeedConfig, Error> {
        match self.configs.get(asset.clone()) {
            Some(config) => Ok(config),
            None => {
                let config = read_price_feeds(self.env, asset)?;
                self.configs.set(asset.clone(), config.clone());

                Ok(config)
            }
        }
    }

    fn price(&mut self, asset: &Address, config: &PriceFeedConfig) -> Result<i128, Error> {
        let price = self.prices.get(asset.clone());

        match price {
            Some(price) => Ok(price),
            None => {
                // TODO: TWAP
                // TODO: Medianizer - implement multiple price oracles for an asset
                // TODO: and calculate median price

                let feeds_len = config.feeds.len();

                let mut price = 0;

                for i in 0..feeds_len {
                    price = self.twap(&config.feeds.get_unchecked(i))?;
                }

                // let price_data = client
                //     .lastprice(&config.feed_asset.clone().into())
                //     .ok_or(Error::NoPriceForAsset)?;

                let actual_price = price;
                // FixedI128::from_rational(price_data.price, 10i128.pow(config.feed_decimals))
                //     .ok_or(Error::InvalidAssetPrice)?
                //     .into_inner();

                self.prices.set(asset.clone(), actual_price);

                Ok(actual_price)
            }
        }
    }

    fn twap(&mut self, config: &PriceFeed) -> Result<i128, Error> {
        let client = PriceFeedClient::new(self.env, &config.feed);

        let prices = client
            .prices(&config.feed_asset.clone().into(), &config.twap_records)
            .ok_or(Error::NoPriceForAsset)?;

        if prices.is_empty() {
            return Err(Error::NoPriceForAsset);
        }

        let prices_len = prices.len();

        if prices_len == 1 {
            return Ok(prices.first_unchecked().price);
        }

        let curr_time = self.env.ledger().timestamp();

        let mut cum_price = {
            let price_curr = prices.get_unchecked(0);

            let time_delta = curr_time
                .checked_sub(price_curr.timestamp)
                .ok_or(Error::MathOverflowError)?;

            if time_delta.eq(&0) {
                price_curr.price
            } else {
                price_curr
                    .price
                    .checked_mul(time_delta.into())
                    .ok_or(Error::MathOverflowError)?
            }
        };

        for i in 1..prices_len {
            let price_prev = prices.get_unchecked(i - 1);
            let price_curr = prices.get_unchecked(i);

            let time_delta = price_prev
                .timestamp
                .checked_sub(price_curr.timestamp)
                .ok_or(Error::MathOverflowError)?;

            let tw_price = price_curr
                .price
                .checked_mul(time_delta.into())
                .ok_or(Error::MathOverflowError)?;

            cum_price = cum_price
                .checked_add(tw_price)
                .ok_or(Error::MathOverflowError)?;
        }

        let twap_time = curr_time
            .checked_sub(prices.last_unchecked().timestamp)
            .ok_or(Error::MathOverflowError)?;

        let twap_price = cum_price
            .checked_div(twap_time.into())
            .ok_or(Error::MathOverflowError)?;

        Ok(twap_price)
    }
}
