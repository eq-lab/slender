use pool_interface::types::{error::Error, ir_params::IRParams};
use soroban_sdk::{Address, Env};

use crate::{
    event,
    storage::{write_admin, write_flash_loan_fee, write_ir_params, write_treasury},
};

use super::validation::{require_admin_not_exist, require_valid_ir_params};

pub fn initialize(
    env: &Env,
    admin: &Address,
    treasury: &Address,
    flash_loan_fee: u32,
    ir_params: &IRParams,
) -> Result<(), Error> {
    require_admin_not_exist(env);
    require_valid_ir_params(env, ir_params);

    write_admin(env, admin);
    write_treasury(env, treasury);
    write_ir_params(env, ir_params);
    write_flash_loan_fee(env, flash_loan_fee);

    event::initialized(env, admin, treasury, ir_params);

    Ok(())
}
