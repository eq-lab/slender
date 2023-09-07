use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env, Vec};

use crate::storage::write_price_feed;
use crate::types::price_provider::PriceProvider;

use super::utils::validation::require_admin;

pub fn set_price_feed(env: &Env, feed: &Address, assets: &Vec<Address>) -> Result<(), Error> {
    require_admin(env)?;
    PriceProvider::new(env, feed);

    write_price_feed(env, feed, assets);

    Ok(())
}
