use pool_interface::types::error::Error;
use soroban_sdk::Env;

use crate::storage::write_flash_loan_fee;

use super::utils::validation::require_admin;

pub fn set_flash_loan_fee(env: &Env, fee: u32) -> Result<(), Error> {
    require_admin(env)?;
    write_flash_loan_fee(env, fee);
    Ok(())
}
