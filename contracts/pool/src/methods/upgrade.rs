use pool_interface::types::{error::Error, permission::Permission};
use soroban_sdk::{Address, BytesN, Env};

use super::utils::validation::require_permission;

pub fn upgrade(env: &Env, who: &Address, new_wasm_hash: &BytesN<32>) -> Result<(), Error> {
    require_permission(&env, who, &Permission::UpgradePoolWasm).unwrap();

    env.deployer()
        .update_current_contract_wasm(new_wasm_hash.clone());

    Ok(())
}
