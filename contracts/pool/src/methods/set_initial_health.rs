use common::PERCENTAGE_FACTOR;
use pool_interface::types::error::Error;
use soroban_sdk::{panic_with_error, Env};

use crate::storage::write_initial_health;

use super::utils::validation::require_admin;

pub fn set_initial_health(env: &Env, value: u32) -> Result<(), Error> {
    require_admin(env)?;

    // validate initial health
    if !(1..=PERCENTAGE_FACTOR).contains(&value) {
        panic_with_error!(env, Error::ValidateInitialHealthError);
    }

    write_initial_health(env, value);

    Ok(())
}
