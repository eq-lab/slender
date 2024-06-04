use pool_interface::types::base_asset_config::BaseAssetConfig;
use pool_interface::types::error::Error;
use pool_interface::types::permission::Permission;
use pool_interface::types::pool_config::PoolConfig;
use soroban_sdk::Address;
use soroban_sdk::Env;

use crate::write_base_asset;
use crate::write_flash_loan_fee;
use crate::write_initial_health;
use crate::write_liquidation_protocol_fee;
use crate::write_min_position_amounts;
use crate::write_reserve_timestamp_window;
use crate::write_user_assets_limit;

use super::utils::validation::require_lte_percentage_factor;
use super::utils::validation::require_non_negative;
use super::utils::validation::require_permission;

pub fn set_pool_configuration(env: &Env, who: &Address, config: &PoolConfig) -> Result<(), Error> {
    require_permission(env, who, &Permission::SetPoolConfiguration)?;

    require_lte_percentage_factor(env, config.initial_health);
    require_lte_percentage_factor(env, config.flash_loan_fee);
    require_non_negative(env, config.min_collat_amount);
    require_non_negative(env, config.min_debt_amount);

    let base_asset = &BaseAssetConfig::new(&config.base_asset_address, config.base_asset_decimals);

    write_base_asset(env, base_asset);
    write_initial_health(env, config.initial_health);
    write_reserve_timestamp_window(env, config.timestamp_window);
    write_flash_loan_fee(env, config.flash_loan_fee);
    write_user_assets_limit(env, config.user_assets_limit);
    write_min_position_amounts(env, config.min_collat_amount, config.min_debt_amount);
    write_liquidation_protocol_fee(env, config.liquidation_protocol_fee);

    Ok(())
}
