use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env};

use crate::{read_pool_config, types::price_provider::PriceProvider};

pub fn twap_median_price(env: Env, asset: Address, amount: i128) -> Result<i128, Error> {
    let pool_config = &read_pool_config(&env)?;

    PriceProvider::new(&env, pool_config)?.convert_to_base(&asset, amount)
}
