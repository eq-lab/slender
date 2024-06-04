use pool_interface::types::{error::Error, ir_params::IRParams, permission::Permission};
use soroban_sdk::{Address, Env};

use crate::storage::write_ir_params;

use super::utils::validation::{require_admin, require_permission, require_valid_ir_params};

pub fn set_ir_params(env: &Env, who: &Address, input: &IRParams) -> Result<(), Error> {
    require_permission(&env, who, &Permission::SetIRParams)?;

    require_admin(env)?;
    require_valid_ir_params(env, input);

    write_ir_params(env, input);

    Ok(())
}
