use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env};

use crate::storage::{read_reserve, write_reserve};

use super::utils::validation::require_admin;

pub fn set_base_asset(env: &Env, asset: &Address, is_base: bool) -> Result<(), Error> {
    require_admin(env)?;

    let mut reserve_data = read_reserve(env, asset)?;
    reserve_data.configuration.is_base_asset = is_base;

    write_reserve(env, asset, &reserve_data);

    Ok(())
}
