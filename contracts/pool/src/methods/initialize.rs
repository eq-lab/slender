use pool_interface::types::permission::Permission;
use pool_interface::types::pool_config::PoolConfig;
use pool_interface::types::{error::Error, ir_params::IRParams};
use soroban_sdk::{vec, Address, Env};

use crate::{event, write_permission_owners};

use super::set_ir_params::set_ir_params;
use super::set_pool_configuration::set_pool_configuration;
use super::utils::validation::require_permissions_owner_not_exist;

pub fn initialize(
    env: &Env,
    permission_owner: &Address,
    ir_params: &IRParams,
    pool_config: &PoolConfig,
) -> Result<(), Error> {
    require_permissions_owner_not_exist(env);

    let owners = vec![env, permission_owner.clone()];
    write_permission_owners(env, &owners, &Permission::Permission);
    set_ir_params(env, None, ir_params)?;
    set_pool_configuration(env, None, pool_config)?;

    event::initialized(env, permission_owner, ir_params, pool_config);

    Ok(())
}
