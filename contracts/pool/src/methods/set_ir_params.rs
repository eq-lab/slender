use pool_interface::types::{error::Error, ir_params::IRParams};
use soroban_sdk::Env;

use crate::storage::write_ir_params;

use super::utils::validation::{require_admin, require_valid_ir_params};

pub fn set_ir_params(env: &Env, input: &IRParams) -> Result<(), Error> {
    require_admin(env)?;
    require_valid_ir_params(env, input); //@audit here we validate parameters! But nowhere else...

    write_ir_params(env, input);
    //@audit should we be able to change that immediately? Should we emit an event?
    Ok(())
}
