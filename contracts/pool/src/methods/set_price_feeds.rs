use pool_interface::types::error::Error;
use pool_interface::types::price_feed_config_input::PriceFeedConfigInput;
use soroban_sdk::{Env, Vec};

use crate::storage::write_price_feeds;

use super::utils::validation::require_admin;

pub fn set_price_feeds(env: &Env, inputs: &Vec<PriceFeedConfigInput>) -> Result<(), Error> {
    require_admin(env)?;

    write_price_feeds(env, inputs);

    Ok(())
}
