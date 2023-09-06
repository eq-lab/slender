use pool_interface::types::error::Error;
use pool_interface::types::init_reserve_input::InitReserveInput;
use pool_interface::types::reserve_data::ReserveData;
use soroban_sdk::{assert_with_error, Address, BytesN, Env};

use crate::methods::validation::{require_admin, require_uninitialized_reserve};
use crate::storage::{read_ir_params, read_reserves, write_reserve, write_reserves};

use super::rate::calc_accrued_rates;

pub fn init_reserve(env: &Env, asset: &Address, input: &InitReserveInput) -> Result<(), Error> {
    require_admin(env)?;
    require_uninitialized_reserve(env, asset);

    let mut reserve_data = ReserveData::new(env, input);
    let mut reserves = read_reserves(env);
    let reserves_len = reserves.len();

    assert_with_error!(
        env,
        reserves_len <= u8::MAX as u32,
        Error::ReservesMaxCapacityExceeded
    );

    let id = reserves_len as u8;

    reserve_data.id = BytesN::from_array(env, &[id; 1]);
    reserves.push_back(asset.clone());

    write_reserves(env, &reserves);
    write_reserve(env, asset, &reserve_data);

    Ok(())
}

pub fn recalculate_reserve_data(
    env: &Env,
    asset: &Address,
    reserve: &ReserveData,
    s_token_supply: i128,
    debt_token_supply: i128,
) -> Result<ReserveData, Error> {
    let current_time = env.ledger().timestamp();
    let elapsed_time = current_time
        .checked_sub(reserve.last_update_timestamp)
        .ok_or(Error::AccruedRateMathError)?;

    if elapsed_time == 0 || s_token_supply == 0 {
        return Ok(reserve.clone());
    }

    let ir_params = read_ir_params(env)?;
    let accrued_rates = calc_accrued_rates(
        s_token_supply,
        debt_token_supply,
        elapsed_time,
        ir_params,
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
