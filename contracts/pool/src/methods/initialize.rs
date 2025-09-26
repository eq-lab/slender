use pool_interface::types::error::Error;
use pool_interface::types::pool_config::PoolConfig;
use soroban_sdk::{Address, Env};

use crate::event::InitializedEvent;
use crate::storage::write_admin;

use super::set_pool_configuration::set_pool_configuration;
use super::utils::validation::require_admin_not_exist;

pub fn initialize(env: &Env, admin: &Address, pool_config: &PoolConfig) -> Result<(), Error> {
    require_admin_not_exist(env);

    write_admin(env, admin);

    set_pool_configuration(env, pool_config, false)?;

    InitializedEvent {
        admin: admin.clone(),
        base_asset_address: pool_config.base_asset_address.clone(),
        ir_alpha: pool_config.ir_alpha,
        ir_initial_rate: pool_config.ir_initial_rate,
        ir_max_rate: pool_config.ir_max_rate,
        ir_scaling_coeff: pool_config.ir_scaling_coeff,
        base_asset_decimals: pool_config.base_asset_decimals,
        initial_health: pool_config.initial_health,
        grace_period: pool_config.grace_period,
        timestamp_window: pool_config.timestamp_window,
        flash_loan_fee: pool_config.flash_loan_fee,
        user_assets_limit: pool_config.user_assets_limit,
        min_collat_amount: pool_config.min_collat_amount,
        min_debt_amount: pool_config.min_debt_amount,
        liquidation_protocol_fee: pool_config.liquidation_protocol_fee,
    }
    .publish(env);

    Ok(())
}
