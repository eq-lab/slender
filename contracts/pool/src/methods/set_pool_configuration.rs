use pool_interface::types::base_asset_config::BaseAssetConfig;
use pool_interface::types::error::Error;
use pool_interface::types::pool_config::PoolConfig;
use soroban_sdk::Env;

use crate::write_base_asset;
use crate::write_flash_loan_fee;
use crate::write_initial_health;
use crate::write_reserve_timestamp_window;
use crate::write_user_assets_limit;

use super::utils::validation::require_admin;

pub fn set_pool_configuration(env: &Env, config: &PoolConfig) -> Result<(), Error> {
    require_admin(env)?;

    let base_asset = &BaseAssetConfig::new(&config.base_asset_address, config.base_asset_decimals);

    write_base_asset(env, base_asset);
    write_initial_health(env, config.initial_health);
    write_reserve_timestamp_window(env, config.timestamp_window);
    write_flash_loan_fee(env, config.flash_loan_fee);
    write_user_assets_limit(env, config.user_assets_limit);

    Ok(())
}
