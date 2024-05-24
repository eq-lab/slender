use pool_interface::types::error::Error;
use pool_interface::types::pool_config::PoolConfig;
use soroban_sdk::Env;

use crate::read_base_asset;
use crate::read_flash_loan_fee;
use crate::read_initial_health;
use crate::read_reserve_timestamp_window;
use crate::read_user_assets_limit;

pub fn pool_configuration(env: &Env) -> Result<PoolConfig, Error> {
    let base_asset = read_base_asset(env)?;

    Ok(PoolConfig {
        base_asset_address: base_asset.address,
        base_asset_decimals: base_asset.decimals,
        flash_loan_fee: read_flash_loan_fee(env),
        initial_health: read_initial_health(env)?,
        user_assets_limit: read_user_assets_limit(env),
        timestamp_window: read_reserve_timestamp_window(env),
    })
}
