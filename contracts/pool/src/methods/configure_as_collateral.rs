use pool_interface::types::collateral_params_input::CollateralParamsInput;
use pool_interface::types::error::Error;
use pool_interface::types::permission::Permission;
use soroban_sdk::{Address, Env};

use crate::event;
use crate::storage::{read_reserve, write_reserve};

use super::utils::validation::{
    require_permission, require_unique_liquidation_order, require_valid_collateral_params,
};

pub fn configure_as_collateral(
    env: &Env,
    who: &Address,
    asset: &Address,
    params: &CollateralParamsInput,
) -> Result<(), Error> {
    require_permission(env, who, &Permission::CollateralReserveParams)?;
    require_valid_collateral_params(env, params);
    require_unique_liquidation_order(env, asset, params.pen_order)?;

    let mut reserve = read_reserve(env, asset)?;
    reserve.update_collateral_config(params);

    write_reserve(env, asset, &reserve);
    event::collat_config_change(env, asset, params);

    Ok(())
}
