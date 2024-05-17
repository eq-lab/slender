use pool_interface::types::error::Error;
use soroban_sdk::Env;

use crate::storage::write_reserve_timestamp_window;

use super::utils::validation::require_admin;

pub fn set_reserve_timestamp_window(env: &Env, window: u64) -> Result<(), Error> {
    require_admin(env)?; //@audit does not panic. 
    //@audit note to self: we check the admin signed on the invocation parameters ... but not if they are CORRECT or MAKE SENSE.
    write_reserve_timestamp_window(env, window);
    //@audit note to self: do we need to pause the protocol here? Or recompute some values?
    Ok(())
}
