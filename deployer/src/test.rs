#![cfg(test)]

extern crate std;

use crate::{Deployer, DeployerClient};
use soroban_sdk::{testutils::Address as _, Address, Bytes, BytesN, Env};

mod pool {
    soroban_sdk::contractimport!(file = "../target/wasm32-unknown-unknown/release/pool.wasm");
}

mod s_token {
    soroban_sdk::contractimport!(file = "../target/wasm32-unknown-unknown/release/s_token.wasm");
}

#[test]
fn deploy_pool_and_s_token() {
    let env = Env::default();
    let client = DeployerClient::new(&env, &env.register_contract(None, Deployer));

    // Deploy pool
    let pool_contract_id = {
        // Install the WASM code to be deployed from the deployer contract.
        let pool_wasm_hash = env.install_contract_wasm(pool::WASM);

        // Deploy contract using deployer, and include an init function to call.
        let salt = BytesN::from_array(&env, &[5; 32]);
        let pool_admin = Address::random(&env);

        let (contract_id, init_result) = client.deploy_pool(&salt, &pool_wasm_hash, &pool_admin);
        assert!(init_result.is_void());

        contract_id
    };

    // Invoke contract to check that it is initialized.
    let pool_client = pool::Client::new(&env, &pool_contract_id);

    // Deploy s-token
    let s_token_contract_id = {
        let s_token_wasm_hash = env.install_contract_wasm(s_token::WASM);

        let decimal = 7u32;
        let treasury = Address::random(&env);
        let underlying_asset = Address::random(&env);
        let name = Bytes::from_slice(&env, b"name");
        let symbol = Bytes::from_slice(&env, b"symbol");

        let (contract_id, init_result) = client.deploy_s_token(
            &BytesN::from_array(&env, &[1; 32]),
            &s_token_wasm_hash,
            &decimal,
            &name,
            &symbol,
            &pool_client.address,
            &treasury,
            &underlying_asset,
        );
        assert!(init_result.is_void());

        contract_id
    };

    let _s_token_client = s_token::Client::new(&env, &s_token_contract_id);
}
