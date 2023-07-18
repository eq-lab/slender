#![deny(warnings)]
#![no_std]

use crate::price_provider::PriceProvider;
use common::{FixedI128, PERCENTAGE_FACTOR};
use debt_token_interface::DebtTokenClient;
use pool_interface::*;
use s_token_interface::STokenClient;
use soroban_sdk::{
    assert_with_error, contractimpl, panic_with_error, token, unwrap::UnwrapOptimized, Address,
    BytesN, Env, Vec,
};

mod event;
mod price_provider;
mod rate;
mod storage;

use crate::storage::*;

#[allow(dead_code)] //TODO: remove after full implement validate_borrow
#[derive(Debug, Clone, Copy)]
struct AccountData {
    /// Total collateral expresed in XLM
    collateral: i128,
    /// Total debt expressed in XLM
    debt: i128,
    /// Debt with liquidation penalty
    liquidation_debt: Option<i128>,
    /// Net position value in XLM
    npv: i128,
}

impl AccountData {
    pub fn is_good_position(&self) -> bool {
        self.npv > 0
    }
}

pub struct LendingPool;

#[contractimpl]
impl LendingPoolTrait for LendingPool {
    /// Initializes the contract with the specified admin address.
    ///
    /// # Arguments
    ///
    /// - admin - The address of the admin for the contract.
    /// - ir_params - The interest rate parameters to set.
    ///
    /// # Panics
    ///
    /// Panics with `AlreadyInitialized` if the admin key already exists in storage.
    ///
    fn initialize(env: Env, admin: Address, ir_params: IRParams) -> Result<(), Error> {
        if has_admin(&env) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }
        Self::require_valid_ir_params(&env, &ir_params);

        write_admin(&env, admin);
        write_ir_params(&env, &ir_params);

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
    /// - Panics with `ReserveAlreadyInitialized` if the specified asset key already exists in storage.
    /// - Panics with `MustBeLtePercentageFactor` if initial_rate or max_rate are invalid.
    /// - Panics with `MustBeLtPercentageFactor` if scaling_coeff is invalid.
    /// - Panics if the caller is not the admin.
    ///
    fn init_reserve(env: Env, asset: Address, input: InitReserveInput) -> Result<(), Error> {
        Self::require_admin(&env)?;
        Self::require_uninitialized_reserve(&env, &asset);

        let mut reserve_data = ReserveData::new(&env, input);
        let mut reserves = read_reserves(&env);
        let reserves_len = reserves.len();

        assert_with_error!(
            &env,
            reserves_len <= u8::MAX as u32,
            Error::ReservesMaxCapacityExceeded
        );

        let id = reserves_len as u8;
        reserve_data.id = BytesN::from_array(&env, &[id; 1]);
        reserves.push_back(asset.clone());

        write_reserves(&env, &reserves);
        write_reserve(&env, asset, &reserve_data);

        Ok(())
    }

    /// Updates an interest rate parameters.
    ///
    /// # Arguments
    ///
    /// - input - The interest rate parameters to set.
    ///
    /// # Panics
    ///
    /// - Panics with `Uninitialized` if the admin or ir_params key are not exist in storage.
    /// - Panics with `MustBeLtePercentageFactor` if alpha or initial_rate are invalid.
    /// - Panics with `MustBeGtPercentageFactor` if max_rate is invalid.
    /// - Panics with `MustBeLtPercentageFactor` if scaling_coeff is invalid.
    /// - Panics if the caller is not the admin.
    ///
    fn set_ir_params(env: Env, input: IRParams) -> Result<(), Error> {
        Self::require_admin(&env)?;
        Self::require_valid_ir_params(&env, &input);

        write_ir_params(&env, &input);

        Ok(())
    }

    /// Retrieves the interest rate parameters.
    ///
    /// # Returns
    ///
    /// Returns the interest rate parameters if set, or None otherwise.
    ///
    fn get_ir_params(env: Env) -> Option<IRParams> {
        read_ir_params(&env).ok()
    }

    /// Enable borrowing
    ///
    /// # Arguments
    ///
    ///  - asset - target asset
    ///  - enabled - enable/disable borrow flag
    ///
    /// # Errors
    ///
    /// - NoReserveExistForAsset
    ///
    /// # Panics
    ///
    /// - If the caller is not the admin.
    ///
    fn enable_borrowing_on_reserve(env: Env, asset: Address, enabled: bool) -> Result<(), Error> {
        Self::require_admin(&env)?;

        let mut reserve = read_reserve(&env, asset.clone())?;
        reserve.configuration.borrowing_enabled = enabled;
        write_reserve(&env, asset.clone(), &reserve);

        if enabled {
            event::borrowing_enabled(&env, asset);
        } else {
            event::borrowing_disabled(&env, asset);
        }

        Ok(())
    }

    /// Configures the reserve collateralization parameters
    /// all the values are expressed in percentages with two decimals of precision.
    ///
    /// # Arguments
    ///
    /// - asset - The address of asset that should be set as collateral
    /// - params - Collateral parameters
    ///
    /// # Panics
    ///
    /// - Panics with `MustBeLtePercentageFactor` if discount is invalid.
    /// - Panics with `MustBeGtPercentageFactor` if liq_bonus is invalid.
    /// - Panics with `MustBePositive` if liq_cap is invalid.
    /// - Panics with `NoReserveExistForAsset` if no reserve exists for the specified asset.
    /// - Panics if the caller is not the admin.
    ///
    fn configure_as_collateral(
        env: Env,
        asset: Address,
        params: CollateralParamsInput,
    ) -> Result<(), Error> {
        Self::require_admin(&env)?;
        Self::require_valid_collateral_params(&env, &params);

        let mut reserve = read_reserve(&env, asset.clone())?;
        reserve.update_collateral_config(params);

        write_reserve(&env, asset.clone(), &reserve);
        event::collat_config_change(&env, asset, params);

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

    /// Sets the price feed oracle address for a given assets.
    ///
    /// # Arguments
    ///
    /// - feed - The contract address of the price feed oracle.
    /// - assets - The collection of assets associated with the price feed.
    ///
    /// # Panics
    ///
    /// - Panics with `Uninitialized` if the admin key is not exist in storage.
    /// - Panics if the caller is not the admin.
    ///
    fn set_price_feed(env: Env, feed: Address, assets: Vec<Address>) -> Result<(), Error> {
        Self::require_admin(&env)?;
        PriceProvider::new(&env, &feed);

        write_price_feed(&env, feed, &assets);

        Ok(())
    }

    /// Retrieves the price feed oracle address for a given asset.
    ///
    /// # Arguments
    ///
    /// - asset - The address of the asset associated with the price feed.
    ///
    /// # Returns
    ///
    /// Returns the price feed oracle contract id associated with the asset if set, or None otherwise.
    ///
    fn get_price_feed(env: Env, asset: Address) -> Option<Address> {
        read_price_feed(&env, asset).ok()
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
        Self::require_not_paused(&env)?;

        let mut reserve = read_reserve(&env, asset.clone())?;
        Self::validate_deposit(&env, &reserve, amount);

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
            reserve.collat_accrued_rate,
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

    /// Validates and finalize a sToken transfer.
    /// Only callable by the aToken.
    ///
    /// # Arguments
    ///
    /// - asset - The address of the underlying asset of the sToken
    /// - from - The address from which sTokens are transferred
    /// - to - The address receiving sTokens
    /// - amount - The amount being trasferred
    /// - balance_from_before - The sToken balance of the `from` before the transfer
    /// - balance_to_before - The sToken balance of the `to` before the transfer
    ///
    /// # Errors
    ///
    /// Returns `NoReserveExistForAsset` if no reserve exists for the specified asset.
    /// Returns `Paused` if contract is paused
    /// Returns `BadPostion` if the `from` address is subject to liquidation after the transfer
    /// Returns `MathOverflowError' if an overflow occurs when calculating the `from` position.
    ///
    /// # Panics
    ///
    /// Panics if the caller is not the sToken contract.
    fn finalize_transfer(
        env: Env,
        asset: Address,
        from: Address,
        to: Address,
        amount: i128,
        balance_from_before: i128,
        balance_to_before: i128,
    ) -> Result<(), Error> {
        // TODO: maybe check with callstack?
        let reserve = read_reserve(&env, asset.clone())?;
        reserve.s_token_address.require_auth();
        Self::require_not_paused(&env)?;
        let balance_from_after = balance_from_before
            .checked_sub(amount)
            .ok_or(Error::InvalidAmount)?;

        let mut from_config = read_user_config(&env, from.clone())?;
        let reserves = read_reserves(&env);
        let account_data = Self::calc_account_data(
            &env,
            from.clone(),
            Some((reserve.s_token_address, balance_from_after)),
            &from_config,
            &reserves,
            false,
        )?;
        Self::require_good_position(account_data)?;

        if from != to {
            let reserve_id = read_reserve(&env, asset.clone())?.get_id();
            if balance_from_before.checked_sub(amount) == Some(0) {
                from_config.set_using_as_collateral(&env, reserve_id, false);
                write_user_config(&env, from.clone(), &from_config);
                event::reserve_used_as_collateral_disabled(&env, from, asset.clone());
            }

            if balance_to_before == 0 && amount != 0 {
                let mut user_config = read_user_config(&env, to.clone())?;
                user_config.set_using_as_collateral(&env, reserve_id, true);
                write_user_config(&env, to.clone(), &user_config);
                event::reserve_used_as_collateral_enabled(&env, to, asset);
            }
        }

        Ok(())
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
    ///
    fn withdraw(
        env: Env,
        who: Address,
        asset: Address,
        amount: i128,
        to: Address,
    ) -> Result<(), Error> {
        who.require_auth();
        Self::require_not_paused(&env)?;

        let mut reserve = read_reserve(&env, asset.clone())?;

        let s_token = STokenClient::new(&env, &reserve.s_token_address);
        let who_balance = s_token.balance(&who);
        let amount_to_withdraw = if amount == i128::MAX {
            who_balance
        } else {
            amount
        };

        let mut user_config: UserConfiguration = read_user_config(&env, who.clone())?;
        Self::validate_withdraw(
            &env,
            who.clone(),
            &reserve,
            &user_config,
            amount_to_withdraw,
            who_balance,
        );

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

        // amount_to_burn = amount_to_withdraw / liquidity_index
        let amount_to_burn = FixedI128::from_inner(reserve.collat_accrued_rate)
            .recip_mul_int(amount_to_withdraw)
            .ok_or(Error::MathOverflowError)?;
        s_token.burn(&who, &amount_to_burn, &amount_to_withdraw, &to);

        event::withdraw(&env, who, asset, to, amount_to_withdraw);
        Ok(())
    }

    /// Allows users to borrow a specific `amount` of the reserve underlying asset, provided that the borrower
    /// already deposited enough collateral
    ///
    /// # Arguments
    /// - who The address of user performing borrowing
    /// - asset The address of the underlying asset to borrow
    /// - amount The amount to be borrowed
    ///
    /// # Panics
    /// - Panics when caller is not authorized as who
    /// - Panics if user balance doesn't meet requirements for borrowing an amount of asset
    ///
    fn borrow(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error> {
        who.require_auth();
        Self::require_not_paused(&env)?;

        let mut reserve = read_reserve(&env, asset.clone())?;
        let user_config = read_user_config(&env, who.clone())?;

        Self::validate_borrow(&env, who.clone(), &asset, &reserve, &user_config, amount)?;

        let debt_token = DebtTokenClient::new(&env, &reserve.debt_token_address);
        let is_first_borrowing = debt_token.balance(&who) == 0;
        debt_token.mint(&who, &amount);

        if is_first_borrowing {
            let mut user_config = user_config;
            user_config.set_borrowing(&env, reserve.get_id(), true);
            write_user_config(&env, who.clone(), &user_config);
        }

        reserve.update_interest_rate();
        write_reserve(&env, asset.clone(), &reserve);

        let s_token = STokenClient::new(&env, &reserve.s_token_address);
        s_token.transfer_underlying_to(&who, &amount);

        event::borrow(&env, who, asset, amount);

        Ok(())
    }

    fn set_pause(env: Env, value: bool) -> Result<(), Error> {
        Self::require_admin(&env)?;
        write_pause(&env, value);
        Ok(())
    }

    fn paused(env: Env) -> bool {
        paused(&env)
    }

    fn get_account_data(env: Env, who: Address) -> Result<(i128, i128, i128), Error> {
        let AccountData {
            collateral,
            debt,
            npv,
            liquidation_debt: _,
        } = Self::calc_account_data(
            &env,
            who.clone(),
            None,
            &read_user_config(&env, who)?,
            &read_reserves(&env),
            false,
        )?;
        Ok((collateral, debt, npv))
    }

    /// Liqudate a bad position with NPV less or equal to 0.
    /// The caller (liquidator) covers amount of debt of the user getting liquidated, and receives
    /// a proportionally amount of the `collateralAsset` plus a bonus to cover market risk.
    ///
    /// # Arguments
    ///
    /// - liquidator The caller, that covers debt and take collateral with bonus
    /// - who The address of the user whose position will be liquidated
    /// - receive_stoken `true` if the liquidators wants to receive the collateral sTokens, `false` if he wants
    /// to receive the underlying asset
    fn liquidate(
        env: Env,
        liquidator: Address,
        who: Address,
        receive_stoken: bool,
    ) -> Result<(), Error> {
        liquidator.require_auth();
        let reserves = read_reserves(&env);
        let mut user_config = read_user_config(&env, who.clone())?;
        let account_data =
            Self::calc_account_data(&env, who.clone(), None, &user_config, &reserves, true)?;
        if account_data.is_good_position() {
            return Err(Error::GoodPosition);
        }

        let liquidation_debt = account_data
            .liquidation_debt
            .expect("pool: liquidation flag in calc_account_data");

        Self::do_liquidate(
            &env,
            liquidator,
            who.clone(),
            reserves,
            &mut user_config,
            liquidation_debt,
            receive_stoken,
        )?;
        event::liquidation(&env, who, account_data.debt, liquidation_debt);

        Ok(())
    }

    #[cfg(any(test, feature = "testutils"))]
    fn set_accrued_rates(
        env: Env,
        asset: Address,
        collat_accrued_rate: Option<i128>,
        debt_accrued_rate: Option<i128>,
    ) -> Result<(), Error> {
        let mut reserve_data = read_reserve(&env, asset.clone())?;

        if !collat_accrued_rate.is_none() {
            reserve_data.collat_accrued_rate = collat_accrued_rate.unwrap();
        }

        if !debt_accrued_rate.is_none() {
            reserve_data.debt_accrued_rate = debt_accrued_rate.unwrap();
        }

        write_reserve(&env, asset, &reserve_data);

        Ok(())
    }
}

impl LendingPool {
    fn require_admin(env: &Env) -> Result<(), Error> {
        let admin: Address = read_admin(env)?;
        admin.require_auth();
        Ok(())
    }

    fn require_valid_ir_params(env: &Env, params: &IRParams) {
        Self::require_lte_percentage_factor(env, params.initial_rate);
        Self::require_gt_percentage_factor(env, params.max_rate);
        Self::require_lt_percentage_factor(env, params.scaling_coeff);
    }

    fn require_valid_collateral_params(env: &Env, params: &CollateralParamsInput) {
        Self::require_lte_percentage_factor(env, params.discount);
        Self::require_gt_percentage_factor(env, params.liq_bonus);
        Self::require_positive(env, params.liq_cap);
    }

    fn require_uninitialized_reserve(env: &Env, asset: &Address) {
        assert_with_error!(
            env,
            !has_reserve(env, asset.clone()),
            Error::ReserveAlreadyInitialized
        );
    }

    fn require_lte_percentage_factor(env: &Env, value: u32) {
        assert_with_error!(
            env,
            value <= PERCENTAGE_FACTOR,
            Error::MustBeLtePercentageFactor
        );
    }

    fn require_lt_percentage_factor(env: &Env, value: u32) {
        assert_with_error!(
            env,
            value < PERCENTAGE_FACTOR,
            Error::MustBeLtPercentageFactor
        );
    }

    fn require_gt_percentage_factor(env: &Env, value: u32) {
        assert_with_error!(
            env,
            value > PERCENTAGE_FACTOR,
            Error::MustBeGtPercentageFactor
        );
    }

    fn require_positive(env: &Env, value: i128) {
        assert_with_error!(env, value > 0, Error::MustBePositive);
    }

    fn do_deposit(
        env: &Env,
        who: &Address,
        s_token_address: &Address,
        asset: &Address,
        amount: i128,
        collat_accrued_rate: i128,
    ) -> Result<bool, Error> {
        let token = token::Client::new(env, asset);
        token.transfer(who, s_token_address, &amount);

        let s_token = s_token_interface::STokenClient::new(env, s_token_address);
        let is_first_deposit = s_token.balance(who) == 0;

        // amount_to_mint = amount / collat_accrued_rate
        let amount_to_mint = FixedI128::from_inner(collat_accrued_rate)
            .recip_mul_int(amount)
            .ok_or(Error::MathOverflowError)?;
        s_token.mint(who, &amount_to_mint);
        Ok(is_first_deposit)
    }

    fn validate_deposit(env: &Env, reserve: &ReserveData, amount: i128) {
        assert_with_error!(env, amount > 0, Error::InvalidAmount);
        let flags = reserve.configuration.get_flags();
        assert_with_error!(env, flags.is_active, Error::NoActiveReserve);
        assert_with_error!(env, !flags.is_frozen, Error::ReserveFrozen);
    }

    fn validate_withdraw(
        env: &Env,
        who: Address,
        reserve: &ReserveData,
        user_config: &UserConfiguration,
        amount: i128,
        balance: i128,
    ) {
        assert_with_error!(env, amount > 0, Error::InvalidAmount);
        let flags = reserve.configuration.get_flags();
        assert_with_error!(env, flags.is_active, Error::NoActiveReserve);
        assert_with_error!(env, amount <= balance, Error::NotEnoughAvailableUserBalance);

        let reserves = read_reserves(env);
        // TODO: fix calc_account_data with balance after withdraw
        let mb_account_data =
            Self::calc_account_data(env, who, None, user_config, &reserves, false);
        match mb_account_data {
            Ok(account_data) => {
                assert_with_error!(env, account_data.is_good_position(), Error::BadPosition)
            }
            Err(e) => assert_with_error!(env, true, e),
        }

        //balance_decrease_allowed()
    }

    fn validate_borrow(
        env: &Env,
        who: Address,
        asset: &Address,
        reserve: &ReserveData,
        user_config: &UserConfiguration,
        amount_to_borrow: i128,
    ) -> Result<(), Error> {
        let asset_price = Self::get_asset_price(env, asset.clone())?;
        let amount_in_xlm = asset_price
            .mul_int(amount_to_borrow)
            .ok_or(Error::ValidateBorrowMathError)?;

        assert_with_error!(
            env,
            amount_to_borrow > 0 && amount_in_xlm > 0,
            Error::InvalidAmount
        );
        let flags = reserve.configuration.get_flags();
        assert_with_error!(env, flags.is_active, Error::NoActiveReserve);
        assert_with_error!(env, !flags.is_frozen, Error::ReserveFrozen);
        assert_with_error!(env, flags.borrowing_enabled, Error::BorrowingNotEnabled);

        let reserves = &read_reserves(env);
        // TODO: check calc_account_data with balances after borrow
        let account_data =
            Self::calc_account_data(env, who, None, user_config, reserves, false)?;

        assert_with_error!(
            env,
            account_data.npv >= amount_in_xlm,
            Error::CollateralNotCoverNewBorrow
        );

        //TODO: complete validation after rate implementation
        Self::require_good_position(account_data)?;

        Ok(())
    }

    fn calc_account_data(
        env: &Env,
        who: Address,
        mb_who_balance: Option<(Address, i128)>,
        user_config: &UserConfiguration,
        reserves: &Vec<Address>,
        liquidation: bool,
    ) -> Result<AccountData, Error> {
        if user_config.is_empty() {
            return Ok(AccountData {
                collateral: 0,
                debt: 0,
                liquidation_debt: liquidation.then_some(0),
                npv: 0,
            });
        }

        let mut total_collateral_in_xlm: i128 = 0;
        let mut total_debt_in_xlm: i128 = 0;
        let mut total_liquidation_debt_in_xlm: i128 = 0;
        let reserves_len =
            u8::try_from(reserves.len()).map_err(|_| Error::ReservesMaxCapacityExceeded)?;

        // calc collateral and debt expressed in XLM token
        for i in 0..reserves_len {
            if !user_config.is_using_as_collateral_or_borrowing(env, i) {
                continue;
            }

            //TODO: avoid unwrap
            let curr_reserve_asset = reserves.get(i.into()).unwrap().unwrap();
            let curr_reserve = read_reserve(env, curr_reserve_asset.clone())?;

            let reserve_price = Self::get_asset_price(env, curr_reserve_asset.clone())?;

            if user_config.is_using_as_collateral(env, i) {
                let coll_coeff = Self::get_collateral_coeff(env, &curr_reserve)?;

                let who_balance: i128 = match mb_who_balance.clone() {
                    Some((address, balance)) if address == curr_reserve.s_token_address.clone() => {
                        balance
                    }
                    _ => STokenClient::new(env, &curr_reserve.s_token_address).balance(&who),
                };

                let discount = FixedI128::from_percentage(curr_reserve.configuration.discount)
                    .ok_or(Error::CalcAccountDataMathError)?;
                let compounded_balance = discount
                    .mul_int(
                        coll_coeff
                            .mul_int(who_balance)
                            .ok_or(Error::CalcAccountDataMathError)?,
                    )
                    .ok_or(Error::CalcAccountDataMathError)?;

                let liquidity_balance_in_xlm = reserve_price
                    .mul_int(compounded_balance)
                    .ok_or(Error::CalcAccountDataMathError)?;

                total_collateral_in_xlm = total_collateral_in_xlm
                    .checked_add(liquidity_balance_in_xlm)
                    .ok_or(Error::CalcAccountDataMathError)?;
            }

            if user_config.is_borrowing(env, i) {
                let debt_coeff = Self::get_debt_coeff(env, &curr_reserve)?;

                let debt_token = token::Client::new(env, &curr_reserve.debt_token_address);
                let compounded_balance = debt_coeff
                    .mul_int(debt_token.balance(&who))
                    .ok_or(Error::CalcAccountDataMathError)?;

                let debt_balance_in_xlm = reserve_price
                    .mul_int(compounded_balance)
                    .ok_or(Error::CalcAccountDataMathError)?;

                total_debt_in_xlm = total_debt_in_xlm
                    .checked_add(debt_balance_in_xlm)
                    .ok_or(Error::CalcAccountDataMathError)?;

                if liquidation {
                    let liq_bonus =
                        FixedI128::from_percentage(curr_reserve.configuration.liq_bonus)
                            .ok_or(Error::CalcAccountDataMathError)?;
                    let liquidation_debt = liq_bonus
                        .mul_int(debt_balance_in_xlm)
                        .ok_or(Error::CalcAccountDataMathError)?;
                    total_liquidation_debt_in_xlm = total_liquidation_debt_in_xlm
                        .checked_add(liquidation_debt)
                        .ok_or(Error::CalcAccountDataMathError)?;
                }
            }
        }

        let npv = total_collateral_in_xlm
            .checked_sub(total_debt_in_xlm)
            .ok_or(Error::CalcAccountDataMathError)?;

        Ok(AccountData {
            collateral: total_collateral_in_xlm,
            debt: total_debt_in_xlm,
            liquidation_debt: liquidation.then_some(total_liquidation_debt_in_xlm),
            npv,
        })
    }

    /// Returns price of asset expressed in XLM token and denominator 10^decimals
    fn get_asset_price(env: &Env, asset: Address) -> Result<FixedI128, Error> {
        let price_feed = read_price_feed(env, asset.clone())?;
        let provider = PriceProvider::new(env, &price_feed);
        provider
            .get_price(&asset)
            .ok_or(Error::NoPriceForAsset)
            .map(|price_data| {
                FixedI128::from_rational(price_data.price, price_data.decimals)
                    .ok_or(Error::AssetPriceMathError)
            })?
    }

    fn get_collateral_coeff(_env: &Env, reserve: &ReserveData) -> Result<FixedI128, Error> {
        Ok(FixedI128::from_inner(reserve.collat_accrued_rate))
    }

    fn get_debt_coeff(_env: &Env, reserve: &ReserveData) -> Result<FixedI128, Error> {
        Ok(FixedI128::from_inner(reserve.debt_accrued_rate))
    }

    fn require_not_paused(env: &Env) -> Result<(), Error> {
        if paused(env) {
            return Err(Error::Paused);
        }

        Ok(())
    }

    fn require_good_position(account_data: AccountData) -> Result<(), Error> {
        if !account_data.is_good_position() {
            return Err(Error::BadPosition);
        }

        Ok(())
    }

    fn do_liquidate(
        env: &Env,
        liquidator: Address,
        who: Address,
        reserves: Vec<Address>,
        user_config: &mut UserConfiguration,
        liquidation_debt: i128,
        get_stoken: bool,
    ) -> Result<(), Error> {
        let mut sorted_collat_reserves = Vec::new(env);
        let mut debt_reserves = Vec::new(env);
        // soroban_sdk::Vec doesn't have built-in sort or something like binary_search_by_key
        // so here a little hack that uses second Vec with discounts that tracks indexes for `collat_reserves`
        let mut sorted_discounts = Vec::new(env);
        for i in 0..reserves.len() {
            let asset = reserves.get_unchecked(i).unwrap_optimized();
            if user_config.is_using_as_collateral_or_borrowing(env, i as u8) {
                let reserve = read_reserve(env, asset.clone())?;
                if !reserve.configuration.is_active {
                    return Err(Error::NoActiveReserve);
                }
                if user_config.is_borrowing(env, i as u8) {
                    debt_reserves.push_back(reserve);
                } else if user_config.is_using_as_collateral(env, i as u8) {
                    let idx = match sorted_discounts.binary_search(reserve.configuration.discount) {
                        Ok(exists) => exists,
                        Err(new) => new,
                    };
                    sorted_discounts.insert(idx, reserve.configuration.discount);
                    sorted_collat_reserves.insert(idx, reserve);
                }
            }
        }

        let mut debt_to_cover = liquidation_debt;
        for reserve in sorted_collat_reserves {
            if debt_to_cover == 0 {
                break;
            }

            let reserve = reserve.unwrap_optimized();
            let s_token = STokenClient::new(env, &reserve.s_token_address);
            let underlying_asset = s_token.underlying_asset();
            let coll_coeff = Self::get_collateral_coeff(env, &reserve)?;
            let s_token_balance = s_token.balance(&who);
            let compounded_balance = coll_coeff
                .mul_int(s_token_balance)
                .ok_or(Error::LiquidateMathError)?;

            let price = Self::get_asset_price(env, underlying_asset.clone())?;
            let collateral_in_xlm = price
                .mul_int(compounded_balance)
                .ok_or(Error::LiquidateMathError)?;

            let withdraw_amount_in_xlm = collateral_in_xlm.min(debt_to_cover);
            // no overflow as withdraw_amount_in_xlm guaranteed less or equal than debt_to_cover
            debt_to_cover -= withdraw_amount_in_xlm;

            let (s_token_amount, underlying_amount) = if withdraw_amount_in_xlm != collateral_in_xlm
            {
                let underlying_amount = price
                    .recip_mul_int(withdraw_amount_in_xlm)
                    .ok_or(Error::LiquidateMathError)?;
                let s_token_amount = coll_coeff
                    .recip_mul_int(underlying_amount)
                    .ok_or(Error::LiquidateMathError)?;
                (s_token_amount, underlying_amount)
            } else {
                (s_token_balance, compounded_balance)
            };

            if get_stoken {
                s_token.transfer_on_liquidation(&who, &liquidator, &s_token_amount);
            } else {
                s_token.burn(&who, &s_token_amount, &underlying_amount, &liquidator);
            }

            if s_token_balance == s_token_amount {
                user_config.set_using_as_collateral(env, reserve.get_id(), false);
                event::reserve_used_as_collateral_disabled(env, who.clone(), underlying_asset);
            }
        }

        if debt_to_cover != 0 {
            return Err(Error::NotEnoughCollateral);
        }

        for reserve in debt_reserves {
            let reserve = reserve.unwrap_optimized();
            let s_token = STokenClient::new(env, &reserve.s_token_address);
            let underlying_asset = token::Client::new(env, &s_token.underlying_asset());
            let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
            let debt_amount = &debt_token.balance(&who);
            underlying_asset.transfer(&liquidator, &reserve.s_token_address, debt_amount);
            debt_token.burn(&who, debt_amount);
            user_config.set_borrowing(env, reserve.get_id(), false);
        }

        write_user_config(env, who, user_config);

        Ok(())
    }
}

#[cfg(test)]
mod test;
