#![cfg(test)]

extern crate std;

use crate::{Deployer, DeployerClient};
use pool_interface::IRParams;
use soroban_sdk::{
    testutils::Address as _, token::Client as TokenClient, Address, Bytes, BytesN, Env,
};

mod pool {
    soroban_sdk::contractimport!(file = "../target/wasm32-unknown-unknown/release/pool.wasm");
}

mod s_token {
    soroban_sdk::contractimport!(file = "../target/wasm32-unknown-unknown/release/s_token.wasm");
}

mod debt_token {
    soroban_sdk::contractimport!(file = "../target/wasm32-unknown-unknown/release/debt_token.wasm");
}

#[test]
fn deploy_pool_and_s_token() {
    let env = Env::default();
    let client = DeployerClient::new(&env, &env.register_contract(None, Deployer));

    // Deploy pool
    let pool_ir_params = IRParams {
        alpha: 143,
        initial_rate: 200,
        max_rate: 50_000,
        scaling_coeff: 9_000,
    };
    let pool_contract_id = {
        // Install the WASM code to be deployed from the deployer contract.
        let pool_wasm_hash = env.install_contract_wasm(pool::WASM);

        // Deploy contract using deployer, and include an init function to call.
        let salt = BytesN::from_array(&env, &[0; 32]);
        let pool_admin = Address::random(&env);

        let (contract_id, init_result) =
            client.deploy_pool(&salt, &pool_wasm_hash, &pool_admin, &pool_ir_params);
        assert!(init_result.is_void());

        contract_id
    };

    // Invoke contract to check that it is initialized.
    let pool_client = pool::Client::new(&env, &pool_contract_id);
    let underlying_asset = TokenClient::new(
        &env,
        &env.register_stellar_asset_contract(Address::random(&env)),
    );
    // Deploy s-token
    let s_token_contract_id = {
        let s_token_wasm_hash = env.install_contract_wasm(s_token::WASM);

        let treasury = Address::random(&env);
        let name = Bytes::from_slice(&env, b"name");
        let symbol = Bytes::from_slice(&env, b"symbol");

        let (contract_id, init_result) = client.deploy_s_token(
            &BytesN::from_array(&env, &[1; 32]),
            &s_token_wasm_hash,
            &name,
            &symbol,
            &pool_client.address,
            &treasury,
            &underlying_asset.address,
        );
        assert!(init_result.is_void());

        contract_id
    };

    let _s_token_client = s_token::Client::new(&env, &s_token_contract_id);

    // Deploy debt token
    let debt_token_contract_id = {
        let debt_token_wasm_hash = env.install_contract_wasm(debt_token::WASM);

        let name = Bytes::from_slice(&env, b"name");
        let symbol = Bytes::from_slice(&env, b"symbol");

        let (contract_id, init_result) = client.deploy_debt_token(
            &BytesN::from_array(&env, &[2; 32]),
            &debt_token_wasm_hash,
            &name,
            &symbol,
            &pool_client.address,
            &underlying_asset.address,
        );
        assert!(init_result.is_void());

        contract_id
    };

    let _debt_token_client = debt_token::Client::new(&env, &debt_token_contract_id);
    let ir_params = pool_client.get_ir_params().unwrap();

    assert_eq!(pool_ir_params.alpha, ir_params.alpha);
    assert_eq!(pool_ir_params.initial_rate, ir_params.initial_rate);
    assert_eq!(pool_ir_params.max_rate, ir_params.max_rate);
    assert_eq!(pool_ir_params.scaling_coeff, ir_params.scaling_coeff);
}
