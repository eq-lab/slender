use pool_interface::types::error::Error;
use soroban_sdk::Env;

use crate::storage::write_initial_health;

use super::utils::validation::require_admin;

pub fn set_initial_health(env: &Env, value: u32) -> Result<(), Error> {
    require_admin(env)?;

    write_initial_health(env, value);

    Ok(())
}
