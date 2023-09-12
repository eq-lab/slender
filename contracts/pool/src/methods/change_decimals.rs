use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env};

use crate::storage::{read_reserve, write_reserve};

use super::utils::validation::require_admin;

pub fn change_decimals(env: &Env, asset: &Address, decimals: u32) -> Result<(), Error> {
    require_admin(env)?;
    let mut reserve_data = read_reserve(env, asset)?;
    reserve_data.configuration.decimals = decimals;

    write_reserve(env, asset, &reserve_data);

    Ok(())
}
