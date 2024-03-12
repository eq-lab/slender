use pool_interface::types::error::Error;
use s_token_interface::STokenClient;
use soroban_sdk::{Address, BytesN, Env};

use crate::storage::read_reserve;

use super::utils::{get_fungible_lp_tokens::get_fungible_lp_tokens, validation::require_admin};

pub fn upgrade_s_token(
    env: &Env,
    asset: &Address,
    new_wasm_hash: &BytesN<32>,
) -> Result<(), Error> {
    require_admin(env).unwrap();

    let reserve = read_reserve(env, asset)?;
    let (s_token_address, _) = get_fungible_lp_tokens(&reserve)?;
    let s_token = STokenClient::new(env, s_token_address);
    s_token.upgrade(new_wasm_hash);

    Ok(())
}
