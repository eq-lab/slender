use pool_interface::types::{error::Error, reserve_type::ReserveType};
use s_token_interface::STokenClient;
use soroban_sdk::{Address, BytesN, Env};

use crate::storage::read_reserve;

use super::utils::validation::{require_admin, require_fungible_reserve};

pub fn upgrade_s_token(
    env: &Env,
    asset: &Address,
    new_wasm_hash: &BytesN<32>,
) -> Result<(), Error> {
    require_admin(env).unwrap();

    let reserve = read_reserve(env, asset)?;
    require_fungible_reserve(env, &reserve);
    if let ReserveType::Fungible(s_token_address, _) = reserve.reserve_type {
        let s_token = STokenClient::new(env, &s_token_address);
        s_token.upgrade(new_wasm_hash);
    }

    Ok(())
}
