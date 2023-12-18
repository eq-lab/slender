use pool_interface::types::error::Error;
use pool_interface::types::price_feed_input::PriceFeedInput;
use soroban_sdk::{Env, Vec};

use crate::storage::write_price_feed;

use super::utils::validation::require_admin;

pub fn set_price_feed(env: &Env, inputs: &Vec<PriceFeedInput>) -> Result<(), Error> {
    require_admin(env)?;

    write_price_feed(env, inputs);

    Ok(())
}
