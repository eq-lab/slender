use debt_token_interface::DebtTokenClient;
use pool_interface::types::error::Error;
use soroban_sdk::{Address, BytesN, Env};

use crate::storage::read_reserve;

use super::utils::validation::require_admin;

pub fn upgrade_debt_token(
    env: &Env,
    asset: &Address,
    new_wasm_hash: &BytesN<32>,
) -> Result<(), Error> {
    require_admin(env).unwrap();

    let reserve = read_reserve(env, asset)?;
    let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
    debt_token.upgrade(new_wasm_hash);

    Ok(())
}
