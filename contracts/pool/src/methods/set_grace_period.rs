use pool_interface::types::{error::Error, permission::Permission};
use soroban_sdk::{Address, Env};

use crate::{
    methods::utils::validation::require_non_zero_grace_period, read_pause_info, write_pause_info,
};

use super::utils::validation::require_permission;

pub fn set_grace_period(env: Env, who: &Address, grace_period: u64) -> Result<(), Error> {
    require_permission(&env, who, &Permission::SetGracePeriod)?;

    require_non_zero_grace_period(&env, grace_period);

    let mut pause_info = read_pause_info(&env)?;
    pause_info.grace_period_secs = grace_period;
    write_pause_info(&env, pause_info);

    Ok(())
}
