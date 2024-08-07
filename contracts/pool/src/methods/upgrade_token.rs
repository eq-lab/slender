use debt_token_interface::DebtTokenClient;
use pool_interface::types::error::Error;
use s_token_interface::STokenClient;
use soroban_sdk::{Address, BytesN, Env};

use crate::storage::read_reserve;

use super::utils::validation::require_admin;

pub fn upgrade_token(
    env: &Env,
    asset: &Address,
    new_wasm_hash: &BytesN<32>,
    s_token: bool,
) -> Result<(), Error> {
    require_admin(env)?;

    let reserve = read_reserve(env, asset)?;
    let (s_token_address, debt_token_address) = reserve.get_fungible()?;

    if s_token {
        STokenClient::new(env, s_token_address).upgrade(new_wasm_hash)
    } else {
        DebtTokenClient::new(env, debt_token_address).upgrade(new_wasm_hash);
    }

    Ok(())
}
