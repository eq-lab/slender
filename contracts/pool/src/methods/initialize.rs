use pool_interface::types::pool_config::PoolConfig;
use pool_interface::types::{error::Error, ir_params::IRParams};
use soroban_sdk::{Address, Env};

use crate::event;
use crate::storage::write_admin;

use super::set_ir_params::set_ir_params;
use super::set_pool_configuration::set_pool_configuration;
use super::utils::validation::require_admin_not_exist;

pub fn initialize(
    env: &Env,
    admin: &Address,
    ir_params: &IRParams,
    pool_config: &PoolConfig,
) -> Result<(), Error> {
    require_admin_not_exist(env);

    write_admin(env, admin);

    set_ir_params(env, ir_params, false)?;
    set_pool_configuration(env, pool_config, false)?;

    event::initialized(env, admin, ir_params, pool_config);

    Ok(())
}
