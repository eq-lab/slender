#![deny(warnings)]
#![no_std]

use crate::storage::*;
use common_token::{
    balance::*, require_nonnegative_amount, require_positive_amount, storage::*,
    verify_caller_is_pool,
};
use pool_interface::LendingPoolClient;
use s_token_interface::STokenTrait;
use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env, String};
use soroban_token_sdk::metadata::TokenMetadata;

mod event;
mod storage;

#[cfg(test)]
mod tests;

#[contract]
pub struct SToken;

#[contractimpl]
impl STokenTrait for SToken {
    /// Initializes the Stoken contract.
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
    /// Panics with if the specified decimal value exceeds the maximum value of u8.
    /// Panics with if the contract has already been initialized.
    /// Panics if name or symbol is empty
    ///
    fn initialize(e: Env, name: String, symbol: String, pool: Address, underlying_asset: Address) {
        if name.len() == 0 {
            panic!("s-token: no name");
        }

        if symbol.len() == 0 {
            panic!("s-token: no symbol");
        }

        if has_pool(&e) {
            panic!("s-token: already initialized")
        }

        write_pool(&e, &pool);
        write_underlying_asset(&e, &underlying_asset);

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

    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        verify_caller_is_pool(&env);

        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    fn version() -> u32 {
        1
    }

    /// Returns the amount of tokens that the `spender` is allowed to withdraw from the `from` address.
    ///
    /// # Arguments
    ///
    /// - from - The address of the token owner.
    /// - spender - The address of the spender.
    ///
    /// # Returns
    ///
    /// The amount of tokens that the `spender` is allowed to withdraw from the `from` address.
    ///
    fn allowance(e: Env, from: Address, spender: Address) -> i128 {
        read_allowance(&e, from, spender).amount
    }

    /// Set the allowance for a spender to withdraw from the `from` address by a specified amount of tokens.
    ///
    /// # Arguments
    ///
    /// - from - The address of the token owner.
    /// - spender - The address of the spender.
    /// - amount - The amount of tokens to increase the allowance by.
    /// - expiration_ledger - The time when allowance will be expired.
    ///
    /// # Panics
    ///
    /// Panics if the caller is not authorized.
    /// Panics if the amount is negative.
    /// Panics if the updated allowance exceeds the maximum value of i128.
    ///
    fn approve(e: Env, from: Address, spender: Address, amount: i128, expiration_ledger: u32) {
        from.require_auth();

        require_nonnegative_amount(amount);

        write_allowance(&e, from.clone(), spender.clone(), amount, expiration_ledger);
        event::approve(&e, from, spender, amount, expiration_ledger);
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
    fn balance(e: Env, id: Address) -> i128 {
        read_balance(&e, id)
    }

    /// Returns the spendable balance of tokens for a specified id.
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
    fn spendable_balance(e: Env, id: Address) -> i128 {
        read_balance(&e, id)
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
    fn authorized(e: Env, id: Address) -> bool {
        is_authorized(&e, id)
    }

    /// Transfers a specified amount of tokens from one account (`from`) to another account (`to`).
    ///
    /// # Arguments
    ///
    /// - from - The address of the token sender.
    /// - to - The address of the token recipient.
    /// - amount - The amount of tokens to transfer.
    ///
    /// # Panics
    ///
    /// Panics if the caller (`from`) is not authorized.
    /// Panics if the amount is negative.
    ///
    fn transfer(e: Env, from: Address, to: Address, amount: i128) {
        from.require_auth();
        require_positive_amount(amount);

        do_transfer(&e, from, to, amount, true);
    }

    /// Transfers a specified amount of tokens from the from account to the to account on behalf of the spender account.
    ///
    /// # Arguments
    ///
    /// - spender - The address of the account that is authorized to spend tokens.
    /// - from - The address of the token sender.
    /// - to - The address of the token recipient.
    /// - amount - The amount of tokens to transfer.
    ///
    /// # Panics
    ///
    /// Panics if the spender is not authorized.
    /// Panics if the spender is not allowed to spend `amount`.
    /// Panics if the amount is negative.
    ///
    fn transfer_from(e: Env, spender: Address, from: Address, to: Address, amount: i128) {
        spender.require_auth();
        require_positive_amount(amount);
        spend_allowance(&e, from.clone(), spender, amount);

        do_transfer(&e, from, to, amount, true);
    }

    fn burn_from(_e: Env, _spender: Address, _from: Address, _amount: i128) {
        unimplemented!();
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
    fn clawback(e: Env, from: Address, amount: i128) {
        verify_caller_is_pool(&e);

        spend_balance(&e, from.clone(), amount);
        add_total_supply(&e, amount.checked_neg().expect("s-token: no overflow"));
        event::clawback(&e, from, amount);
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

    /// Mints a specified amount of tokens for a given `id` and returns total supply
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
    fn mint(e: Env, to: Address, amount: i128) {
        let pool = verify_caller_is_pool(&e);

        do_mint(&e, to.clone(), amount);
        event::mint(&e, pool, to, amount);
    }

    /// Burns a specified amount of tokens from the from account and returns total supply
    ///
    /// # Arguments
    ///
    /// - from - The address of the token holder to burn tokens from.
    /// - amount_to_burn - The amount of tokens to burn.
    /// - amount_to_withdraw - The amount of underlying token to withdraw.
    /// - to - The address who accepts underlying token.
    ///
    /// # Panics
    ///
    /// Panics if the amount_to_burn is negative.
    /// Panics if the caller is not the pool associated with this token.
    ///
    fn burn(e: Env, from: Address, amount_to_burn: i128, amount_to_withdraw: i128, to: Address) {
        verify_caller_is_pool(&e);

        do_burn(&e, from.clone(), amount_to_burn, amount_to_withdraw, to);
        event::burn(&e, from, amount_to_burn);
    }

    /// Returns the number of decimal places used by the token.
    ///
    /// # Returns
    ///
    /// The number of decimal places used by the token.
    ///
    fn decimals(e: Env) -> u32 {
        read_decimal(&e)
    }

    /// Returns the name of the token.
    ///
    /// # Returns
    ///
    /// The name of the token as a `soroban_sdk::Bytes` value.
    ///
    fn name(e: Env) -> String {
        read_name(&e)
    }

    /// Returns the symbol of the token.
    ///
    /// # Returns
    ///
    /// The symbol of the token as a `soroban_sdk::Bytes` value.
    ///
    fn symbol(e: Env) -> String {
        read_symbol(&e)
    }

    /// Returns the total supply of tokens.
    ///
    /// # Returns
    ///
    /// The total supply of tokens.
    ///
    fn total_supply(e: Env) -> i128 {
        read_total_supply(&e)
    }

    /// Transfers tokens during a liquidation.
    ///
    /// # Arguments
    ///
    /// - from - The address of the sender.
    /// - to - The address of the recipient.
    /// - amount - The amount of tokens to transfer.
    ///
    /// # Panics
    ///
    /// Panics if caller is not associated pool.
    ///
    fn transfer_on_liquidation(e: Env, from: Address, to: Address, amount: i128) {
        verify_caller_is_pool(&e);
        require_positive_amount(amount);

        do_transfer(&e, from, to, amount, false);
    }

    /// Transfers the underlying asset to the specified recipient.
    ///
    /// # Arguments
    ///
    /// - to - The address of the recipient.
    /// - amount - The amount of underlying asset to transfer.
    ///
    /// # Panics
    ///
    /// Panics if the amount is negative.
    /// Panics if caller is not associated pool.
    ///
    fn transfer_underlying_to(e: Env, to: Address, amount: i128) {
        require_positive_amount(amount);
        verify_caller_is_pool(&e);

        let underlying_asset = read_underlying_asset(&e);
        let current_address = e.current_contract_address();

        let token_client = token::Client::new(&e, &underlying_asset);
        token_client.transfer(&current_address, &to, &amount);

        event::transfer(&e, current_address, to, amount);
    }

    /// Retrieves the address of the underlying asset.
    ///
    /// # Returns
    ///
    /// The address of the underlying asset.
    ///
    fn underlying_asset(e: Env) -> Address {
        read_underlying_asset(&e)
    }

    /// Retrieves the address of the pool.
    ///
    /// # Returns
    ///
    /// The address of the associated pool.
    ///
    fn pool(e: Env) -> Address {
        read_pool(&e)
    }
}

fn do_transfer(e: &Env, from: Address, to: Address, amount: i128, validate: bool) {
    let from_balance_prev = read_balance(e, from.clone());
    let to_balance_prev = read_balance(e, to.clone());

    spend_balance(e, from.clone(), amount);
    receive_balance(e, to.clone(), amount);

    if validate && cfg!(not(feature = "testutils")) {
        let underlying_asset = read_underlying_asset(e);
        let total_supply = read_total_supply(e);
        let pool_client = LendingPoolClient::new(e, &read_pool(e));
        pool_client.finalize_transfer(
            &underlying_asset,
            &from,
            &to,
            &amount,
            &from_balance_prev,
            &to_balance_prev,
            &total_supply,
        );
    }

    event::transfer(e, from, to, amount)
}

fn spend_allowance(e: &Env, from: Address, spender: Address, amount: i128) {
    let allowance = read_allowance(e, from.clone(), spender.clone());
    if allowance.amount < amount {
        panic!("s-token: insufficient allowance");
    }
    write_allowance(
        e,
        from,
        spender,
        allowance.amount - amount,
        allowance.expiration_ledger,
    );
}

/// Makes mint and returns updates total supply
fn do_mint(e: &Env, user: Address, amount: i128) {
    if amount == 0 {
        panic!("s-token: invalid mint amount");
    }

    receive_balance(e, user, amount);
    add_total_supply(e, amount);
}

/// Makes burn and returns updates total supply
fn do_burn(e: &Env, from: Address, amount_to_burn: i128, amount_to_withdraw: i128, to: Address) {
    if amount_to_burn == 0 {
        panic!("s-token: invalid burn amount");
    }

    spend_balance(e, from, amount_to_burn);
    add_total_supply(
        e,
        amount_to_burn.checked_neg().expect("s-token: no overflow"),
    );

    let underlying_asset_client = token::Client::new(e, &read_underlying_asset(e));
    underlying_asset_client.transfer(&e.current_contract_address(), &to, &amount_to_withdraw);
}
