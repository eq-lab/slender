use pool_interface::types::pause_info::PauseInfo;
use pool_interface::types::permission::Permission;
use pool_interface::types::{error::Error, ir_params::IRParams};
use soroban_sdk::{vec, Address, Env};

use crate::storage::{write_flash_loan_fee, write_initial_health, write_ir_params};
use crate::{event, write_pause_info, write_permission_owners};

use super::utils::validation::{
    require_non_zero_grace_period, require_permissions_owner_not_exist, require_valid_ir_params,
};

pub fn initialize(
    env: &Env,
    permission_owner: &Address,
    flash_loan_fee: u32,
    initial_health: u32,
    ir_params: &IRParams,
    grace_period: u64,
) -> Result<(), Error> {
    require_permissions_owner_not_exist(env);
    require_valid_ir_params(env, ir_params);
    require_non_zero_grace_period(env, grace_period);

    let owners = vec![env, permission_owner.clone()];

    write_permission_owners(env, &owners, &Permission::Permisssion);
    write_ir_params(env, ir_params);
    write_flash_loan_fee(env, flash_loan_fee);
    write_initial_health(env, initial_health);
    write_pause_info(
        env,
        PauseInfo {
            paused: false,
            grace_period_secs: grace_period,
            unpaused_at: 0,
        },
    );

    event::initialized(env, permission_owner, ir_params);

    Ok(())
}
