use common::FixedI128;
use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env};

use crate::{storage::read_price_feed, types::price_provider::PriceProvider};

/// Returns price of asset expressed in XLM token and denominator 10^decimals
pub fn get_asset_price(
    env: &Env,
    asset: &Address,
    is_base_asset: bool,
) -> Result<FixedI128, Error> {
    if is_base_asset {
        return Ok(FixedI128::ONE);
    }

    let price_feed = read_price_feed(env, asset)?;
    let provider = PriceProvider::new(env, &price_feed);

    provider.get_price(asset).map(|price_data| {
        FixedI128::from_rational(price_data.price, 10i128.pow(price_data.decimals))
            .ok_or(Error::AssetPriceMathError)
    })?
}
