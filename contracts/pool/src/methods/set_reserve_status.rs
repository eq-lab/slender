use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env};

use crate::event;
use crate::storage::{read_reserve, write_reserve};

use super::utils::validation::require_admin;

pub fn set_reserve_status(env: &Env, asset: &Address, is_active: bool) -> Result<(), Error> {
    require_admin(env)?;

    let mut reserve = read_reserve(env, asset)?;

    reserve.configuration.is_active = is_active;
    write_reserve(env, asset, &reserve);

    if is_active {
        event::reserve_activated(env, asset); //@audit question to self: should we emit an event even if we did not change the activation status of the reserve?
    } else {
        event::reserve_deactivated(env, asset);
    }
    //@audit if we just deactivated a reserve - does this change some user npv status? Should they be given some grace period?
    Ok(())
}
