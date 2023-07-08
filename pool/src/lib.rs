#![deny(warnings)]
#![no_std]

use common::RateMath;
use pool_interface::*;
use price_feed_interface::{PriceFeedClient};
use soroban_sdk::{assert_with_error, contractimpl, panic_with_error, token, Address, BytesN, Env};

mod event;
mod price_provider;
mod storage;

use crate::storage::*;

pub struct LendingPool;

#[contractimpl]
impl LendingPoolTrait for LendingPool {
    /// Initializes the contract with the specified admin address.
    ///
    /// # Arguments
    ///
    /// - admin - The address of the admin for the contract.
    ///
    /// # Panics
    ///
    /// Panics with `AlreadyInitialized` if the admin key already exists in storage.
    ///
    fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if has_admin(&env) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }

        write_admin(&env, admin);

        Ok(())
    }

    /// Initializes a reserve for a given asset.
    ///
    /// # Arguments
    ///
    /// - asset - The address of the asset associated with the reserve.
    /// - input - The input parameters for initializing the reserve.
    ///
    /// # Panics
    ///
    /// - Panics with `Uninitialized` if the admin key is not exist in storage.
    /// - Panics if the caller is not the admin.
    /// - Panics with `ReserveAlreadyInitialized` if the specified asset key already exists in storage.
    ///
    fn init_reserve(env: Env, asset: Address, input: InitReserveInput) -> Result<(), Error> {
        Self::ensure_admin(&env)?;
        // ensure_contract(env, asset)?;
        if has_reserve(&env, asset.clone()) {
            panic_with_error!(&env, Error::ReserveAlreadyInitialized);
        }

        let mut reserve_data = ReserveData::new(&env, input);
        let mut reserves = read_reserves(&env);

        let id = reserves.len() as u8;
        reserve_data.id = BytesN::from_array(&env, &[id; 1]);
        reserves.push_back(asset.clone());

        write_reserves(&env, &reserves);
        write_reserve(&env, asset, &reserve_data);

        Ok(())
    }

    /// Retrieves the reserve data for the specified asset.
    ///
    /// # Arguments
    ///
    /// - asset - The address of the asset associated with the reserve.
    ///
    /// # Returns
    ///
    /// Returns the reserve data for the specified asset if it exists, or None otherwise.
    ///
    fn get_reserve(env: Env, asset: Address) -> Option<ReserveData> {
        read_reserve(&env, asset).ok()
    }

    /// Sets the price feed oracle address.
    ///
    /// # Arguments
    ///
    /// - feed - The contract address of the price feed oracle.
    ///
    /// # Panics
    ///
    /// - Panics with `Uninitialized` if the admin key is not exist in storage.
    /// - Panics if the caller is not the admin.
    ///
    fn set_price_feed(env: Env, feed: Address) -> Result<(), Error> {
        Self::ensure_admin(&env)?;
        PriceFeedClient::new(&env, &feed.clone());

        write_price_feed(&env, feed);

        Ok(())
    }

    /// Retrieves the price feed oracle address.
    ///
    /// # Returns
    ///
    /// Returns the price feed oracle contract id if set, or None otherwise.
    ///
    fn get_price_feed(env: Env) -> Option<Address> {
        read_price_feed(&env).ok()
    }

    /// Deposits a specified amount of an asset into the reserve associated with the asset.
    /// Depositor receives s-tokens according to the current index value.
    ///
    /// # Arguments
    ///
    /// - who - The address of the user making the deposit.
    /// - asset - The address of the asset to be deposited.
    /// - amount - The amount to be deposited.
    ///
    /// # Errors
    ///
    /// Returns `NoReserveExistForAsset` if no reserve exists for the specified asset.
    /// Returns `MathOverflowError' if an overflow occurs when calculating the amount of the s-token to be minted.
    ///
    /// # Panics
    ///
    /// If the caller is not authorized.
    /// If the deposit amount is invalid or does not meet the reserve requirements.
    /// If the reserve data cannot be retrieved from storage.
    ///
    fn deposit(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error> {
        who.require_auth();

        let mut reserve = read_reserve(&env, asset.clone())?;
        Self::validate_deposit(&reserve, &env, amount);

        // Updates the reserve indexes and the timestamp of the update.
        // Implement later with rates.
        reserve.update_state();
        // TODO: write reserve into storage

        let is_first_deposit = Self::do_deposit(
            &env,
            &who,
            &reserve.s_token_address,
            &asset,
            amount,
            reserve.liquidity_index,
        )?;

        if is_first_deposit {
            let mut user_config: UserConfiguration =
                read_user_config(&env, who.clone()).unwrap_or_default();

            user_config.set_using_as_collateral(&env, reserve.get_id(), true);
            write_user_config(&env, who.clone(), &user_config);
            event::reserve_used_as_collateral_enabled(&env, who.clone(), asset.clone());
        }

        event::deposit(&env, who, asset, amount);

        Ok(())
    }

    fn finalize_transfer(
        _asset: Address,
        _from: Address,
        _to: Address,
        _amount: i128,
        _balance_from_before: i128,
        _balance_to_before: i128,
    ) {
        // mock to use in s_token
        // whenNotPaused
    }

    /// Withdraws a specified amount of an asset from the reserve and transfers it to the caller.
    /// Burn s-tokens from depositor according to the current index value.
    ///
    /// # Arguments
    ///
    /// - who - The address of the user making the withdrawal.
    /// - asset - The address of the asset to be withdrawn.
    /// - amount - The amount to be withdrawn. Use i128::MAX to withdraw the maximum available amount.
    /// - to - The address of the recipient of the withdrawn asset.
    ///
    /// # Errors
    ///
    /// Returns `NoReserveExistForAsset` if no reserve exists for the specified asset.
    /// Returns `UserConfigNotExists` if the user configuration does not exist in storage.
    /// Returns `MathOverflowError' if an overflow occurs when calculating the amount of the s-token to be burned.
    ///
    /// # Panics
    ///
    /// Panics if the caller is not authorized.
    /// Panics if the withdrawal amount is invalid or does not meet the reserve requirements.
    fn withdraw(
        env: Env,
        who: Address,
        asset: Address,
        amount: i128,
        to: Address,
    ) -> Result<(), Error> {
        who.require_auth();

        let mut reserve = read_reserve(&env, asset.clone())?;

        let s_token = s_token_interface::STokenClient::new(&env, &reserve.s_token_address);
        let who_balance = s_token.balance(&who);
        let amount_to_withdraw = if amount == i128::MAX {
            who_balance
        } else {
            amount
        };

        Self::validate_withdraw(&reserve, &env, amount_to_withdraw, who_balance);

        let mut user_config: UserConfiguration =
            read_user_config(&env, who.clone()).ok_or(Error::UserConfigNotExists)?;

        reserve.update_state();
        //TODO: update interest rates
        // reserve.update_interest_rates(
        //     asset.clone(),
        //     reserve.s_token_address.clone(),
        //     -amount_to_withdraw,
        // );

        //TODO: save new reserve

        if amount_to_withdraw == who_balance {
            user_config.set_using_as_collateral(&env, reserve.get_id(), false);
            write_user_config(&env, who.clone(), &user_config);
            event::reserve_used_as_collateral_disabled(&env, who.clone(), asset.clone());
        }

        let amount_to_burn = amount_to_withdraw
            .div_rate_floor(reserve.liquidity_index)
            .ok_or(Error::MathOverflowError)?;
        s_token.burn(&who, &amount_to_burn, &amount_to_withdraw, &to);

        event::withdraw(&env, who, asset, to, amount_to_withdraw);
        Ok(())
    }

    #[cfg(any(test, feature = "testutils"))]
    fn set_liq_index(env: Env, asset: Address, value: i128) -> Result<(), Error> {
        let mut reserve_data = read_reserve(&env, asset.clone())?;
        reserve_data.liquidity_index = value;
        write_reserve(&env, asset, &reserve_data);

        Ok(())
    }
}

impl LendingPool {
    fn ensure_admin(env: &Env) -> Result<(), Error> {
        let admin: Address = read_admin(env)?;
        admin.require_auth();
        Ok(())
    }

    fn do_deposit(
        env: &Env,
        who: &Address,
        s_token_address: &Address,
        asset: &Address,
        amount: i128,
        liquidity_index: i128,
    ) -> Result<bool, Error> {
        let token = token::Client::new(env, asset);
        token.transfer(who, s_token_address, &amount);

        let s_token = s_token_interface::STokenClient::new(env, s_token_address);
        let is_first_deposit = s_token.balance(who) == 0;
        let amount_to_mint = amount
            .div_rate_floor(liquidity_index)
            .ok_or(Error::MathOverflowError)?;
        s_token.mint(who, &amount_to_mint);
        Ok(is_first_deposit)
    }

    fn validate_deposit(reserve: &ReserveData, env: &Env, amount: i128) {
        assert_with_error!(env, amount != 0, Error::InvalidAmount);
        let flags = reserve.configuration.get_flags();
        assert_with_error!(env, flags.is_active, Error::NoActiveReserve);
        assert_with_error!(env, !flags.is_frozen, Error::ReserveFrozen);
    }

    fn validate_withdraw(reserve: &ReserveData, env: &Env, amount: i128, balance: i128) {
        assert_with_error!(env, amount != 0, Error::InvalidAmount);
        let flags = reserve.configuration.get_flags();
        assert_with_error!(env, flags.is_active, Error::NoActiveReserve);
        assert_with_error!(env, amount <= balance, Error::NotEnoughAvailableUserBalance);

        //TODO: implement when rates exists
        //balance_decrease_allowed()
    }
}

#[cfg(test)]
mod test;
