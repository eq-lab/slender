use pool_interface::types::base_asset_config::BaseAssetConfig;
use pool_interface::types::error::Error;
use pool_interface::types::pause_info::PauseInfo;
use pool_interface::types::pool_config::PoolConfig;
use soroban_sdk::Env;

use crate::read_pause_info;
use crate::write_base_asset;
use crate::write_flash_loan_fee;
use crate::write_initial_health;
use crate::write_liquidation_protocol_fee;
use crate::write_min_position_amounts;
use crate::write_pause_info;
use crate::write_reserve_timestamp_window;
use crate::write_user_assets_limit;

use super::utils::validation::require_admin;
use super::utils::validation::require_valid_pool_config;

pub fn set_pool_configuration(
    env: &Env,
    config: &PoolConfig,
    check_admin: bool,
) -> Result<(), Error> {
    if check_admin {
        require_admin(env)?;
    }

    require_valid_pool_config(env, config);

    let base_asset = &BaseAssetConfig::new(&config.base_asset_address, config.base_asset_decimals);

    write_base_asset(env, base_asset);
    write_initial_health(env, config.initial_health);
    write_reserve_timestamp_window(env, config.timestamp_window);
    write_flash_loan_fee(env, config.flash_loan_fee);
    write_user_assets_limit(env, config.user_assets_limit);
    write_min_position_amounts(env, config.min_collat_amount, config.min_debt_amount);
    write_liquidation_protocol_fee(env, config.liquidation_protocol_fee);

    let pause_info = read_pause_info(env);
    if pause_info.is_err() {
        write_pause_info(
            env,
            PauseInfo {
                paused: false,
                grace_period_secs: config.grace_period,
                unpaused_at: 0,
            },
        );
    } else {
        let mut pause_info = pause_info.unwrap();
        pause_info.grace_period_secs = config.grace_period;
        write_pause_info(env, pause_info);
    }

    Ok(())
}
