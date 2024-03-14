use core::ops::Div;

use common::FixedI128;
use pool_interface::types::base_asset_config::BaseAssetConfig;
use pool_interface::types::error::Error;
use pool_interface::types::price_feed::PriceFeed;
use pool_interface::types::price_feed_config::PriceFeedConfig;
use pool_interface::types::timestamp_precision::TimestampPrecision;
use price_feed_interface::PriceFeedClient;
use soroban_sdk::{Address, Env, Map, Vec};

use crate::storage::{read_base_asset, read_price_feeds};

pub struct PriceProvider<'a> {
    env: &'a Env,
    base_asset: BaseAssetConfig,
    configs: Map<Address, PriceFeedConfig>,
    prices: Map<Address, i128>,
}

impl<'a> PriceProvider<'a> {
    pub fn new(env: &'a Env) -> Result<Self, Error> {
        let base_asset = read_base_asset(env)?;

        Ok(Self {
            env,
            base_asset,
            configs: Map::new(env),
            prices: Map::new(env),
        })
    }

    pub fn convert_to_base(&mut self, asset: &Address, amount: i128) -> Result<i128, Error> {
        if self.base_asset.address == *asset {
            return Ok(amount);
        }

        let config = self.config(asset)?;
        let median_twap_price = self.price(asset, &config)?;

        median_twap_price
            .mul_int(amount)
            .and_then(|a| FixedI128::from_rational(a, 10i128.pow(config.asset_decimals)))
            .and_then(|a| a.to_precision(self.base_asset.decimals))
            .ok_or(Error::InvalidAssetPrice)
    }

    pub fn convert_from_base(&mut self, asset: &Address, amount: i128) -> Result<i128, Error> {
        if self.base_asset.address == *asset {
            return Ok(amount);
        }

        let config = self.config(asset)?;
        let median_twap_price = self.price(asset, &config)?;

        median_twap_price
            .recip_mul_int(amount)
            .and_then(|a| FixedI128::from_rational(a, 10i128.pow(self.base_asset.decimals)))
            .and_then(|a| a.to_precision(config.asset_decimals))
            .ok_or(Error::InvalidAssetPrice)
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

    fn price(&mut self, asset: &Address, config: &PriceFeedConfig) -> Result<FixedI128, Error> {
        let price = self.prices.get(asset.clone());

        let price = match price {
            Some(price) => price,
            None => {
                let mut sorted_twap_prices = Map::new(self.env);

                for feed in config.feeds.iter() {
                    let twap_price =
                        FixedI128::from_rational(self.twap(&feed)?, 10i128.pow(feed.feed_decimals))
                            .ok_or(Error::MathOverflowError)?
                            .into_inner();

                    sorted_twap_prices.set(twap_price, twap_price);
                }

                let median_twap_price = self.median(&sorted_twap_prices.keys())?;

                self.prices.set(asset.clone(), median_twap_price);

                median_twap_price
            }
        };

        Ok(FixedI128::from_inner(price))
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

        let curr_time = precise_timestamp(self.env, &config.timestamp_precision);

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

    fn median(&mut self, prices: &Vec<i128>) -> Result<i128, Error> {
        let prices_len = prices.len();

        if prices_len == 1 {
            return Ok(prices.first_unchecked());
        }

        let index = prices_len / 2;

        let median_price = if prices_len % 2 == 0 {
            let price_1 = prices.get_unchecked(index - 1);
            let price_2 = prices.get_unchecked(index);

            price_1
                .checked_add(price_2)
                .ok_or(Error::MathOverflowError)?
                .div(2)
        } else {
            prices.get_unchecked(index)
        };

        Ok(median_price)
    }
}

pub(crate) fn precise_timestamp(env: &Env, precision: &TimestampPrecision) -> u64 {
    let secs = env.ledger().timestamp();
    match precision {
        TimestampPrecision::Msec => secs * 1000,
        TimestampPrecision::Sec => secs,
    }
}
