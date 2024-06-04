use pool_interface::types::price_feed_config_input::PriceFeedConfigInput;
use pool_interface::types::{error::Error, permission::Permission};
use soroban_sdk::{Address, Env, Vec};

use crate::storage::write_price_feeds;

use super::utils::validation::require_permission;

pub fn set_price_feeds(
    env: &Env,
    who: &Address,
    inputs: &Vec<PriceFeedConfigInput>,
) -> Result<(), Error> {
    require_permission(&env, who, &Permission::SetPriceFeeds)?;

    write_price_feeds(env, inputs);

    Ok(())
}
