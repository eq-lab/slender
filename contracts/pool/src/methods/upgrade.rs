use pool_interface::types::error::Error;
use soroban_sdk::{BytesN, Env};

use super::utils::validation::require_admin;

pub fn upgrade(env: &Env, new_wasm_hash: &BytesN<32>) -> Result<(), Error> {
    require_admin(env)?;

    env.deployer()
        .update_current_contract_wasm(new_wasm_hash.clone());

    Ok(())
}
