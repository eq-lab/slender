use pool_interface::types::error::Error;
use soroban_sdk::Env;

use crate::storage::write_pause;

use super::validation::require_admin;

pub fn set_pause(env: &Env, value: bool) -> Result<(), Error> {
    require_admin(env)?;
    write_pause(env, value);
    Ok(())
}
