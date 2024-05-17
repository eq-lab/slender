use pool_interface::types::error::Error;
use soroban_sdk::{BytesN, Env};

use super::utils::validation::require_admin;

pub fn upgrade(env: &Env, new_wasm_hash: &BytesN<32>) -> Result<(), Error> {
    require_admin(env)?; //@audit note to self: if admin is set incorrectly in the beginning we can not fix it. Should probably make sure it's set by 2-step process. 

    env.deployer()
        .update_current_contract_wasm(new_wasm_hash.clone());

    Ok(())
}
//@audit note to self: checked this contract. Ok.