use common::FixedI128;
use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env, Vec};

use crate::{
    storage::{read_price_feed, write_price_feed},
    types::price_provider::PriceProvider,
};

use super::validation::require_admin;

pub fn set_price_feed(env: &Env, feed: &Address, assets: &Vec<Address>) -> Result<(), Error> {
    require_admin(env)?;
    PriceProvider::new(env, feed);

    write_price_feed(env, feed, assets);

    Ok(())
}

/// Returns price of asset expressed in XLM token and denominator 10^decimals
pub fn get_asset_price(
    env: &Env,
    asset: &Address,
    is_base_asset: bool,
) -> Result<FixedI128, Error> {
    if is_base_asset {
        return Ok(FixedI128::ONE);
    }

    #[cfg(not(feature = "exceeded-limit-fix"))]
    {
        let price_feed = read_price_feed(env, asset)?;
        let provider = PriceProvider::new(env, &price_feed);

        provider.get_price(asset).map(|price_data| {
            FixedI128::from_rational(price_data.price, 10i128.pow(price_data.decimals))
                .ok_or(Error::AssetPriceMathError)
        })?
    }

    #[cfg(feature = "exceeded-limit-fix")]
    {
        Ok(FixedI128::from_inner(read_price(env, asset)))
    }
}
