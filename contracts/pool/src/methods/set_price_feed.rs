use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env, Vec};

use crate::storage::write_price_feed;

use super::utils::validation::require_admin;

pub fn set_price_feed(env: &Env, feed: &Address, assets: &Vec<Address>) -> Result<(), Error> {
    require_admin(env)?;

    write_price_feed(env, feed, assets);

    Ok(())
}
