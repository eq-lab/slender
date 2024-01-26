use debt_token_interface::DebtTokenClient;
use pool_interface::types::{error::Error, reserve_type::ReserveType};
use soroban_sdk::{Address, BytesN, Env};

use crate::storage::read_reserve;

use super::utils::validation::{require_admin, require_fungible_reserve};

pub fn upgrade_debt_token(
    env: &Env,
    asset: &Address,
    new_wasm_hash: &BytesN<32>,
) -> Result<(), Error> {
    require_admin(env).unwrap();

    let reserve = read_reserve(env, asset)?;
    require_fungible_reserve(env, &reserve);
    if let ReserveType::Fungible(_, debt_token_address) = reserve.reserve_type {
        let debt_token = DebtTokenClient::new(env, &debt_token_address);
        debt_token.upgrade(new_wasm_hash);
    }

    Ok(())
}
