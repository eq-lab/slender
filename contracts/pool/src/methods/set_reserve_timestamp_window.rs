use pool_interface::types::error::Error;
use soroban_sdk::Env;

use crate::storage::write_reserve_timestamp_window;

pub fn set_reserve_timestamp_window(env: &Env, window: u64) -> Result<(), Error> {
    write_reserve_timestamp_window(env, window);

    Ok(())
}
