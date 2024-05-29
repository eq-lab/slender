use pool_interface::types::error::Error;
use soroban_sdk::Env;

use crate::{
    methods::utils::validation::{require_admin, require_non_zero_grace_period},
    read_pause_info, write_pause_info,
};

pub fn set_grace_period(env: Env, grace_period: u64) -> Result<(), Error> {
    require_admin(&env)?;
    require_non_zero_grace_period(&env, grace_period);

    let mut pause_info = read_pause_info(&env)?;
    pause_info.grace_period_secs = grace_period;
    write_pause_info(&env, pause_info);

    Ok(())
}
