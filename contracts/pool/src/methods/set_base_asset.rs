use pool_interface::types::{base_asset_config::BaseAssetConfig, error::Error};
use soroban_sdk::{Address, Env};

use crate::storage::write_base_asset;

use super::utils::validation::require_admin;

pub fn set_base_asset(env: &Env, asset: &Address, decimals: u32) -> Result<(), Error> {
    require_admin(env)?;

    write_base_asset(env, &BaseAssetConfig::new(asset, decimals));

    Ok(())
}
