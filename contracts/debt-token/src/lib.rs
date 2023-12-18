#![deny(warnings)]
#![no_std]

use common_token::balance::{add_total_supply, receive_balance, spend_balance};
use common_token::storage::*;
use common_token::verify_caller_is_pool;
use debt_token_interface::DebtTokenTrait;
use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env, String};
use soroban_token_sdk::metadata::TokenMetadata;

mod event;
mod test;

#[contract]
pub struct DebtToken;

#[contractimpl]
impl DebtTokenTrait for DebtToken {
    /// Initializes the Debt token contract.
    ///
    /// # Arguments
    ///
    /// - name - The name of the token.
    /// - symbol - The symbol of the token.
    /// - pool - The address of the pool contract.
    /// - underlying_asset - The address of the underlying asset associated with the token.
    ///
    /// # Panics
    ///
    /// Panics if the specified decimal value exceeds the maximum value of u8.
    /// Panics if the contract has already been initialized.
    /// Panics if name or symbol is empty
    ///
    fn initialize(e: Env, name: String, symbol: String, pool: Address, underlying_asset: Address) {
        if name.len() == 0 {
            panic!("debt-token: no name");
        }

        if symbol.len() == 0 {
            panic!("debt-token: no symbol");
        }

        if has_pool(&e) {
            panic!("debt-token: already initialized");
        }

        write_pool(&e, &pool);

        // it can be optimized by passing decimals as argument
        let token = token::Client::new(&e, &underlying_asset);
        let decimal = token.decimals();

        write_metadata(
            &e,
            TokenMetadata {
                decimal,
                name: name.clone(),
                symbol: symbol.clone(),
            },
        );

        event::initialized(&e, underlying_asset, pool, decimal, name, symbol);
    }

    /// Upgrades the deployed contract wasm preserving the contract id.
    ///
    /// # Arguments
    ///
    /// - new_wasm_hash - The new version of the WASM hash.
    ///
    /// # Panics
    ///
    /// Panics if the caller is not the pool associated with this token.
    ///
    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        verify_caller_is_pool(&env);

        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    /// Returns the current version of the contract.
    fn version() -> u32 {
        1
    }

    /// Returns the balance of tokens for a specified `id`.
    ///
    /// # Arguments
    ///
    /// - id - The address of the account.
    ///
    /// # Returns
    ///
    /// The balance of tokens for the specified `id`.
    ///
    fn balance(env: Env, id: Address) -> i128 {
        read_balance(&env, id)
    }

    ///
    /// # Arguments
    ///
    /// - id - The address of the account.
    ///
    /// # Returns
    ///
    /// The spendable balance of tokens for the specified id.
    ///
    /// Currently the same as `balance(id)`
    fn spendable_balance(env: Env, id: Address) -> i128 {
        read_balance(&env, id)
    }

    /// Checks whether a specified `id` is authorized.
    ///
    /// # Arguments
    ///
    /// - id - The address to check for authorization.
    ///
    /// # Returns
    ///
    /// Returns true if the id is authorized, otherwise returns false
    fn authorized(env: Env, id: Address) -> bool {
        is_authorized(&env, id)
    }

    /// Burns a specified amount of tokens from the from account.
    ///
    /// # Arguments
    ///
    /// - from - The address of the token holder to burn tokens from.
    /// - amount - The amount of tokens to burn.
    ///
    /// # Panics
    ///
    /// Panics if the amount is negative.
    /// Panics if the caller is not the pool associated with this token.
    /// Panics if overflow happens
    ///
    fn burn(env: Env, from: Address, amount: i128) {
        verify_caller_is_pool(&env);

        spend_balance(&env, from.clone(), amount);
        add_total_supply(&env, amount.checked_neg().expect("debt-token: no overflow"));

        event::burn(&env, from, amount);
    }

    fn burn_from(_env: Env, _spender: Address, _from: Address, _amount: i128) {
        unimplemented!();
    }

    /// Sets the authorization status for a specified `id`.
    ///
    /// # Arguments
    ///
    /// - id - The address to set the authorization status for.
    /// - authorize - A boolean value indicating whether to authorize (true) or deauthorize (false) the id.
    ///
    /// # Panics
    ///
    /// Panics if the caller is not the pool associated with this token.
    ///
    fn set_authorized(e: Env, id: Address, authorize: bool) {
        verify_caller_is_pool(&e);

        write_authorization(&e, id.clone(), authorize);
        event::set_authorized(&e, id, authorize);
    }

    /// Mints a specified amount of tokens for a given `id`.
    ///
    /// # Arguments
    ///
    /// - id - The address of the user to mint tokens for.
    /// - amount - The amount of tokens to mint.
    ///
    /// # Panics
    ///
    /// Panics if the amount is negative.
    /// Panics if the caller is not the pool associated with this token.
    ///
    fn mint(env: Env, to: Address, amount: i128) {
        let pool = verify_caller_is_pool(&env);

        receive_balance(&env, to.clone(), amount);
        add_total_supply(&env, amount);
        event::mint(&env, pool, to, amount);
    }

    /// Clawbacks a specified amount of tokens from the from account.
    ///
    /// # Arguments
    ///
    /// - from - The address of the token holder to clawback tokens from.
    /// - amount - The amount of tokens to clawback.
    ///
    /// # Panics
    ///
    /// Panics if the amount is negative.
    /// Panics if the caller is not the pool associated with this token.
    /// Panics if overflow happens
    ///
    fn clawback(env: Env, from: Address, amount: i128) {
        verify_caller_is_pool(&env);

        spend_balance(&env, from.clone(), amount);
        add_total_supply(&env, amount.checked_neg().expect("debt-token: no overflow"));
        event::clawback(&env, from, amount);
    }

    /// Returns the number of decimal places used by the token.
    ///
    /// # Returns
    ///
    /// The number o
    fn decimals(env: Env) -> u32 {
        read_decimal(&env)
    }

    /// Returns the name of the token.
    ///
    /// # Returns
    ///
    /// The name of the token as a `soroban_sdk::Bytes` value.
    ///
    fn name(env: Env) -> String {
        read_name(&env)
    }

    /// Returns the symbol of the token.
    ///
    /// # Returns
    ///
    /// The symbol of the token as a `soroban_sdk::Bytes` value.
    ///
    fn symbol(env: Env) -> String {
        read_symbol(&env)
    }

    /// Returns the total supply of tokens.
    ///
    /// # Returns
    ///
    /// The total supply of tokens.
    ///
    fn total_supply(env: Env) -> i128 {
        read_total_supply(&env)
    }
}
