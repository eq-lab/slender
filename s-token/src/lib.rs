#![deny(warnings)]
#![no_std]

mod event;
mod storage;
mod test;

use crate::storage::*;
use common::RateMath;
use common_token::{check_nonnegative_amount, storage::*, verify_caller_is_pool};
use pool_interface::{LendingPoolClient, ReserveData};
use s_token_interface::STokenTrait;
use soroban_sdk::{contractimpl, token, Address, Bytes, Env};
use soroban_token_sdk::TokenMetadata;

pub struct SToken;

#[contractimpl]
impl STokenTrait for SToken {
    /// Initializes the Stoken contract.
    ///
    /// # Arguments
    ///
    /// - decimal - The number of decimal places for the token.
    /// - name - The name of the token.
    /// - symbol - The symbol of the token.
    /// - pool - The address of the pool contract.
    /// - treasury - The address of the treasury contract.
    /// - underlying_asset - The address of the underlying asset associated with the token.
    ///
    /// # Panics
    ///
    /// Panics with if the specified decimal value exceeds the maximum value of u8.
    /// Panics with if the contract has already been initialized.
    ///
    fn initialize(
        e: Env,
        decimal: u32,
        name: Bytes,
        symbol: Bytes,
        pool: Address,
        treasury: Address,
        underlying_asset: Address,
    ) {
        if decimal > u8::MAX.into() {
            panic!("Decimal must fit in a u8");
        }

        if has_pool(&e) {
            panic!("Already initialized")
        }

        write_pool(&e, &pool);
        write_underlying_asset(&e, &underlying_asset);
        write_treasury(&e, &treasury);

        write_metadata(
            &e,
            TokenMetadata {
                decimal,
                name: name.clone(),
                symbol: symbol.clone(),
            },
        );

        event::initialized(&e, underlying_asset, pool, treasury, decimal, name, symbol);
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
        read_allowance(&e, from, spender)
    }

    /// Increases the allowance for a spender to withdraw from the `from` address by a specified amount of tokens.
    ///
    /// # Arguments
    ///
    /// - from - The address of the token owner.
    /// - spender - The address of the spender.
    /// - amount - The amount of tokens to increase the allowance by.
    ///
    /// # Panics
    ///
    /// Panics if the caller is not authorized.
    /// Panics if the amount is negative.
    /// Panics if the updated allowance exceeds the maximum value of i128.
    ///
    fn increase_allowance(e: Env, from: Address, spender: Address, amount: i128) {
        from.require_auth();

        check_nonnegative_amount(amount);

        let allowance = read_allowance(&e, from.clone(), spender.clone());
        let new_allowance = allowance
            .checked_add(amount)
            .expect("Updated allowance doesn't fit in an i128");

        write_allowance(&e, from.clone(), spender.clone(), new_allowance);
        event::increase_allowance(&e, from, spender, amount);
    }

    /// Decreases the allowance for a spender to withdraw from the `from` address by a specified amount of tokens.
    ///
    /// # Arguments
    ///
    /// - from - The address of the token owner.
    /// - spender - The address of the spender.
    /// - amount - The amount of tokens to decrease the allowance by.
    ///
    /// # Panics
    ///
    /// Panics if the caller is not authorized.
    /// Panics if the amount is negative.
    ///
    fn decrease_allowance(e: Env, from: Address, spender: Address, amount: i128) {
        from.require_auth();

        check_nonnegative_amount(amount);

        let allowance = read_allowance(&e, from.clone(), spender.clone());
        if amount >= allowance {
            write_allowance(&e, from.clone(), spender.clone(), 0);
        } else {
            write_allowance(&e, from.clone(), spender.clone(), allowance - amount);
        }
        event::decrease_allowance(&e, from, spender, amount);
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

    /// Returns the corresponding balance of underlying token for a specified `id`.
    ///
    /// # Arguments
    ///
    /// - id - The address of the user account.
    ///
    /// # Returns
    ///
    /// The underlying balance of tokens for the specified user account.
    ///
    /// # Panics
    ///
    /// Panics if there is an overflow error during the calculation.
    ///
    fn underlying_balance(e: Env, id: Address) -> i128 {
        let (reserve, _) = Self::get_reserve_and_underlying(&e);
        let balance = read_balance(&e, id);
        balance
            .mul_rate_floor(reserve.liquidity_index)
            .unwrap_or_else(|| panic!("s-token: overflow error"))
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
        Self::balance(e, id)
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
        check_nonnegative_amount(amount);

        Self::do_transfer(&e, from, to, amount, true);
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
        check_nonnegative_amount(amount);
        Self::spend_allowance(&e, from.clone(), spender, amount);

        Self::do_transfer(&e, from, to, amount, true);
    }

    fn burn_from(_e: Env, _spender: Address, _from: Address, _amount: i128) {
        panic!("not used")
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
    ///
    fn clawback(e: Env, from: Address, amount: i128) {
        check_nonnegative_amount(amount);
        verify_caller_is_pool(&e);

        Self::spend_balance(&e, from.clone(), amount);
        Self::add_total_supply(&e, -amount);
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
    fn mint(e: Env, to: Address, amount: i128) {
        check_nonnegative_amount(amount);
        let pool = verify_caller_is_pool(&e);

        Self::do_mint(&e, to.clone(), amount);
        event::mint(&e, pool, to, amount);
    }

    /// Burns a specified amount of tokens from the from account.
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
        check_nonnegative_amount(amount_to_burn);
        verify_caller_is_pool(&e);

        Self::do_burn(&e, from.clone(), amount_to_burn, amount_to_withdraw, to);

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
    fn name(e: Env) -> Bytes {
        read_name(&e)
    }

    /// Returns the symbol of the token.
    ///
    /// # Returns
    ///
    /// The symbol of the token as a `soroban_sdk::Bytes` value.
    ///
    fn symbol(e: Env) -> Bytes {
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

    /// Returns the corresponding total supply of the underlying asset.
    ///
    /// # Returns
    ///
    /// The corresponding total supply of the underlying asset.
    fn underlying_total_supply(e: Env) -> i128 {
        let (reserve, _) = Self::get_reserve_and_underlying(&e);
        let total_supply = read_total_supply(&e);

        total_supply
            .mul_rate_floor(reserve.liquidity_index)
            .unwrap_or_else(|| panic!("s-token: overflow error"))
    }

    /// Mints tokens and transfers them to the treasury.
    ///
    /// # Arguments
    ///
    /// - amount - The amount of tokens to mint.
    ///
    /// # Panics
    ///
    /// Panics if the amount is negative.
    /// Panics if caller is not associated pool.
    ///
    fn mint_to_treasury(e: Env, amount: i128) {
        check_nonnegative_amount(amount);
        let pool = verify_caller_is_pool(&e);
        if amount == 0 {
            return;
        }

        let treasury = read_treasury(&e);
        Self::do_mint(&e, treasury.clone(), amount);
        event::mint(&e, pool, treasury, amount);
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

        Self::do_transfer(&e, from, to, amount, false);
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
        check_nonnegative_amount(amount);
        let pool = verify_caller_is_pool(&e);

        let underlying_asset = read_underlying_asset(&e);
        let token_client = token::Client::new(&e, &underlying_asset);
        token_client.transfer(&pool, &to, &amount);

        event::transfer(&e, pool, to, amount);
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

    /// Retrieves the address of the treasury.
    ///
    /// # Returns
    ///
    /// The address of the treasury.
    ///
    fn treasury(e: Env) -> Address {
        read_treasury(&e)
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

impl SToken {
    fn do_transfer(e: &Env, from: Address, to: Address, amount: i128, validate: bool) {
        let underlying_asset = read_underlying_asset(e);

        let from_balance_prev = read_balance(e, from.clone());
        let to_balance_prev = read_balance(e, to.clone());

        Self::spend_balance(e, from.clone(), amount);
        Self::receive_balance(e, to.clone(), amount);

        if validate {
            let pool_client = LendingPoolClient::new(e, &read_pool(e));
            pool_client.finalize_transfer(
                &underlying_asset,
                &from,
                &to,
                &amount,
                &from_balance_prev,
                &to_balance_prev,
            )
        }

        event::transfer(e, from, to, amount)
    }

    fn spend_allowance(e: &Env, from: Address, spender: Address, amount: i128) {
        let allowance = read_allowance(e, from.clone(), spender.clone());
        if allowance < amount {
            panic!("insufficient allowance");
        }
        write_allowance(e, from, spender, allowance - amount);
    }

    fn receive_balance(e: &Env, addr: Address, amount: i128) {
        let balance = read_balance(e, addr.clone());
        if !is_authorized(e, addr.clone()) {
            panic!("can't receive when deauthorized");
        }
        write_balance(e, addr, balance + amount);
    }

    fn spend_balance(e: &Env, addr: Address, amount: i128) {
        let balance = read_balance(e, addr.clone());
        if !is_authorized(e, addr.clone()) {
            panic!("can't spend when deauthorized");
        }
        if balance < amount {
            panic!("insufficient balance");
        }
        write_balance(e, addr, balance - amount);
    }

    fn add_total_supply(e: &Env, amount: i128) {
        let mut total_supply: i128 = read_total_supply(e);
        total_supply = total_supply.checked_add(amount).unwrap();
        if total_supply < 0 {
            panic!("negative total supply");
        }

        write_total_supply(e, total_supply);
    }

    fn do_mint(e: &Env, user: Address, amount: i128) {
        if amount == 0 {
            panic!("s-token: invalid mint amount");
        }

        Self::receive_balance(e, user, amount);
        Self::add_total_supply(e, amount);
    }

    fn do_burn(
        e: &Env,
        from: Address,
        amount_to_burn: i128,
        amount_to_withdraw: i128,
        to: Address,
    ) {
        if amount_to_burn == 0 {
            panic!("s-token: invalid burn amount");
        }

        Self::spend_balance(e, from, amount_to_burn);
        Self::add_total_supply(e, -amount_to_burn);

        let underlying_asset = read_underlying_asset(e);
        let underlying_asset_client = token::Client::new(e, &underlying_asset);
        underlying_asset_client.transfer(&e.current_contract_address(), &to, &amount_to_withdraw);
    }

    fn get_reserve_and_underlying(e: &Env) -> (ReserveData, Address) {
        let pool = read_pool(e);
        let pool_client = pool_interface::LendingPoolClient::new(e, &pool);

        let underlying_asset = read_underlying_asset(e);
        let reserve = pool_client
            .get_reserve(&underlying_asset)
            .unwrap_or_else(|| panic!("s-token: reserve not found for underlying asset"));
        (reserve, underlying_asset)
    }
}
