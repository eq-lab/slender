use pool_interface::types::{error::Error, ir_params::IRParams};
use soroban_sdk::{Address, Env};

use crate::event;
use crate::storage::{
    write_admin, write_flash_loan_fee, write_initial_health, write_ir_params, write_treasury,
};

use super::utils::validation::{require_admin_not_exist, require_valid_ir_params};

//@audit is there a way to revert here without panic and therefore leave the pool created but uninitialized?
pub fn initialize(
    env: &Env,
    admin: &Address,
    treasury: &Address,
    flash_loan_fee: u32,
    initial_health: u32,
    ir_params: &IRParams,
) -> Result<(), Error> {
    require_admin_not_exist(env); //@audit panics
    require_valid_ir_params(env, ir_params); //@audit panics

    write_admin(env, admin); //@audit instance storage
    write_treasury(env, treasury); //@audit instance storage
    write_ir_params(env, ir_params); //@audit instance storage
    write_flash_loan_fee(env, flash_loan_fee); //@audit instance storage
    write_initial_health(env, initial_health); //@audit instance storage

    event::initialized(env, admin, treasury, ir_params);

    Ok(())
}
