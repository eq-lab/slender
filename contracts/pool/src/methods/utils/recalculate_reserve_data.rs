use pool_interface::types::error::Error;
use pool_interface::types::pool_config::PoolConfig;
use pool_interface::types::reserve_data::ReserveData;
use soroban_sdk::{Address, Env};

use crate::storage::write_reserve;

use super::{get_elapsed_time::get_elapsed_time, rate::calc_accrued_rates};

pub fn recalculate_reserve_data(
    env: &Env,
    asset: &Address,
    reserve: &ReserveData,
    pool_config: &PoolConfig,
    s_token_supply: i128,
    debt_token_supply: i128,
) -> Result<ReserveData, Error> {
    let (current_time, elapsed_time) = get_elapsed_time(
        env,
        reserve.last_update_timestamp,
        pool_config.timestamp_window,
    );

    if elapsed_time == 0 || s_token_supply == 0 {
        return Ok(reserve.clone());
    }

    let accrued_rates = calc_accrued_rates(
        s_token_supply,
        debt_token_supply,
        elapsed_time,
        pool_config,
        reserve,
    )
    .ok_or(Error::AccruedRateMathError)?;

    let mut reserve = reserve.clone();
    reserve.lender_ar = accrued_rates.lender_ar.into_inner();
    reserve.borrower_ar = accrued_rates.borrower_ar.into_inner();
    reserve.borrower_ir = accrued_rates.borrower_ir.into_inner();
    reserve.lender_ir = accrued_rates.lender_ir.into_inner();
    reserve.last_update_timestamp = current_time;

    write_reserve(env, asset, &reserve);

    Ok(reserve)
}
