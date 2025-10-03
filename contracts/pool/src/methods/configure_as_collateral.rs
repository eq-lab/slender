use pool_interface::types::collateral_params_input::CollateralParamsInput;
use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env};

use crate::event::CollatConfigChangeEvent;
use crate::storage::{read_reserve, write_reserve};

use super::utils::validation::{
    require_admin, require_unique_liquidation_order, require_valid_collateral_params,
};

pub fn configure_as_collateral(
    env: &Env,
    asset: &Address,
    params: &CollateralParamsInput,
) -> Result<(), Error> {
    require_admin(env)?;
    require_valid_collateral_params(env, params);
    require_unique_liquidation_order(env, asset, params.pen_order)?;

    let mut reserve = read_reserve(env, asset)?;
    reserve.update_collateral_config(params);

    write_reserve(env, asset, &reserve);

    CollatConfigChangeEvent {
        asset: asset.clone(),
        liq_cap: params.liq_cap,
        pen_order: params.pen_order,
        util_cap: params.util_cap,
        discount: params.discount,
    }
    .publish(env);

    Ok(())
}
