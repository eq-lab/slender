use pool_interface::types::error::Error;
use soroban_sdk::Env;

use crate::{read_pause_info, storage::write_pause_info};

use super::utils::validation::require_admin;

pub fn set_pause(env: &Env, value: bool) -> Result<(), Error> {
    require_admin(env)?;
    let mut pause_info = read_pause_info(env);

    if pause_info.paused && !value {
        pause_info.unpaused_at = env.ledger().timestamp();
    }

    pause_info.paused = value;
    write_pause_info(env, pause_info);
    Ok(())
}
