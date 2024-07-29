use pool_interface::types::error::Error;
use pool_interface::types::pool_config::PoolConfig;
use soroban_sdk::Env;

use crate::read_pause_info;
use crate::write_pause_info;
use crate::write_pool_config;

use super::utils::validation::require_admin;
use super::utils::validation::require_valid_pool_config;

pub fn set_pool_configuration(
    env: &Env,
    config: &PoolConfig,
    check_admin: bool,
) -> Result<(), Error> {
    if check_admin {
        require_admin(env)?;
    }

    require_valid_pool_config(env, config);

    write_pool_config(env, config);

    let mut pause_info = read_pause_info(env);
    pause_info.grace_period_secs = config.grace_period;

    write_pause_info(env, pause_info);

    Ok(())
}
