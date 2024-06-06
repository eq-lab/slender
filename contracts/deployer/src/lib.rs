#![deny(warnings)]
#![no_std]

use pool_interface::types::pool_config::PoolConfig;
use soroban_sdk::{
    contract, contractimpl, vec, Address, BytesN, Env, IntoVal, String, Symbol, Val,
};

#[contract]
pub struct Deployer;

#[contractimpl]
impl Deployer {
    /// Deploy the pool contract wasm and after deployment invoke the `initialize` function
    /// of the contract with the given admin address. Returns the contract ID and
    /// result of the `initialize` function.
    #[allow(clippy::too_many_arguments)]
    pub fn deploy_pool(
        env: Env,
        salt: BytesN<32>,
        wasm_hash: BytesN<32>,
        admin: Address,
        pool_config: PoolConfig,
    ) -> (Address, Val) {
        let id = env.deployer().with_current_contract(salt).deploy(wasm_hash);
        let init_fn = Symbol::new(&env, "initialize");
        let init_args = vec![&env, admin.into_val(&env), pool_config.into_val(&env)];
        let res: Val = env.invoke_contract(&id, &init_fn, init_args);
        (id, res)
    }

    /// Deploy the s-token contract wasm and after deployment invoke the `initialize` function
    /// of the contract with the given arguments. Returns the contract ID and
    /// result of the `initialize` function.
    #[allow(clippy::too_many_arguments)]
    pub fn deploy_s_token(
        env: Env,
        salt: BytesN<32>,
        wasm_hash: BytesN<32>,
        name: String,
        symbol: String,
        pool: Address,
        underlying_asset: Address,
    ) -> (Address, Val) {
        let id = env.deployer().with_current_contract(salt).deploy(wasm_hash);
        let init_fn = Symbol::new(&env, "initialize");
        let init_args = vec![
            &env,
            name.into_val(&env),
            symbol.into_val(&env),
            pool.into_val(&env),
            underlying_asset.into_val(&env),
        ];
        let res: Val = env.invoke_contract(&id, &init_fn, init_args);
        (id, res)
    }

    /// Deploy the debt token contract wasm and after deployment invoke the `initialize` function
    /// of the contract with the given arguments. Returns the contract ID and
    /// result of the `initialize` function.
    #[allow(clippy::too_many_arguments)]
    pub fn deploy_debt_token(
        env: Env,
        salt: BytesN<32>,
        wasm_hash: BytesN<32>,
        name: String,
        symbol: String,
        pool: Address,
        underlying_asset: Address,
    ) -> (Address, Val) {
        let id = env.deployer().with_current_contract(salt).deploy(wasm_hash);
        let init_fn = Symbol::new(&env, "initialize");
        let init_args = vec![
            &env,
            name.into_val(&env),
            symbol.into_val(&env),
            pool.into_val(&env),
            underlying_asset.into_val(&env),
        ];
        let res: Val = env.invoke_contract(&id, &init_fn, init_args);
        (id, res)
    }
}

mod test;
