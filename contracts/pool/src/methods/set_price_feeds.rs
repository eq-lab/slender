use pool_interface::types::error::Error;
use pool_interface::types::price_feed_config_input::PriceFeedConfigInput;
use soroban_sdk::{Env, Vec};

use crate::storage::write_price_feeds;

use super::utils::validation::require_admin;

pub fn set_price_feeds(env: &Env, inputs: &Vec<PriceFeedConfigInput>) -> Result<(), Error> {
    require_admin(env)?;
    //@audit note to self: we check the admin signed on the invocation parameters ... but not if they are CORRECT or MAKE SENSE.
    write_price_feeds(env, inputs);

    Ok(())
}
