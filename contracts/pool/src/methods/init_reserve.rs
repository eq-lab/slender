use pool_interface::types::reserve_data::ReserveData;
use pool_interface::types::{error::Error, reserve_type::ReserveType};
use soroban_sdk::{assert_with_error, Address, BytesN, Env};

use crate::storage::{read_reserves, write_reserve, write_reserves};

use super::utils::validation::{require_admin, require_uninitialized_reserve};

pub fn init_reserve(env: &Env, asset: &Address, reserve_type: ReserveType) -> Result<(), Error> {
    require_admin(env)?;
    require_uninitialized_reserve(env, asset);

    let mut reserve_data = ReserveData::new(env, reserve_type);
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
