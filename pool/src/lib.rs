#![deny(warnings)]
#![no_std]

use crate::price_provider::PriceProvider;
use common::{FixedI128, PERCENTAGE_FACTOR};
use debt_token_interface::DebtTokenClient;
use pool_interface::*;
use rate::{calc_accrued_rate_coeff, calc_accrued_rates};
use s_token_interface::STokenClient;
use soroban_sdk::{
    assert_with_error, contractimpl, panic_with_error, token, Address, BytesN, Env, Vec,
};

mod event;
mod price_provider;
mod rate;
mod storage;
#[cfg(test)]
mod test;

use crate::storage::*;

#[allow(dead_code)] //TODO: remove after full implement validate_borrow
#[derive(Debug, Clone, Copy)]
struct AccountData {
    /// Total collateral expresed in XLM
    collateral: i128,
    /// Total debt expressed in XLM
    debt: i128,
    /// Net position value in XLM
    npv: i128,
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

    /// Repays a borrowed amount on a specific reserve, burning the equivalent debt tokens owned when debt exists.
    /// Deposits a specified amount of an asset into the reserve associated with the asset.
    /// Depositor receives s-tokens according to the current index value.
    ///
    ///
    /// # Arguments
    ///
    /// - who - The address of the user making the deposit.
    /// - asset - The address of the asset to be deposited for lend or repay.
    /// - amount - The amount to be repayed/deposited.
    /// - is_deposit - The flag indicating the deposit must be performed after repayment.
    ///
    /// # Errors
    ///
    /// Returns `NoReserveExistForAsset` if no reserve exists for the specified asset.
    /// Returns `MathOverflowError' if an overflow occurs when calculating the amount of tokens.
    ///
    /// # Panics
    ///
    /// If the caller is not authorized.
    /// If the deposit amount is invalid or does not meet the reserve requirements.
    /// If the reserve data cannot be retrieved from storage.
    ///
    fn deposit(
        env: Env,
        who: Address,
        asset: Address,
        amount: i128,
        is_deposit: bool,
    ) -> Result<(), Error> {
        who.require_auth();
        Self::require_not_paused(&env)?;

        let reserve = get_actual_reserve_data(&env, asset.clone())?;
        Self::validate_deposit(&env, &reserve, amount);

        let (remaining_amount, is_repayed) = Self::do_repay(&env, &who, &asset, amount, &reserve)?;

        let is_first_deposit = if is_deposit {
            Self::do_deposit(&env, &who, &asset, remaining_amount, &reserve)?
        } else {
            false
        };

        if is_repayed || is_first_deposit {
            let mut user_config = read_user_config(&env, who.clone()).unwrap_or_default();

            if is_repayed {
                user_config.set_borrowing(&env, reserve.get_id(), false);
            }

            if is_first_deposit {
                user_config.set_using_as_collateral(&env, reserve.get_id(), true);
                event::reserve_used_as_collateral_enabled(&env, who.clone(), asset);
            }

            write_user_config(&env, who, &user_config);
        }

        Ok(())
    }

    /// Callback that should be called by s-token after transfer to ensure user have good position after transfer
    ///
    /// # Arguments
    ///
    /// - asset - underlying asset
    /// - from - address of user who send s-token
    /// - to - user who receive s-token
    /// - amount - sended amount of s-token
    /// - balance_from_before - amount of s-token before transfer on `from` user balance
    /// - balance_to_before - amount of s-token before transfer on `to` user balance
    fn finalize_transfer(
        env: Env,
        asset: Address,
        from: Address,
        _to: Address,
        amount: i128,
        balance_from_before: i128,
        _balance_to_before: i128,
    ) -> Result<(), Error> {
        let reserve = get_actual_reserve_data(&env, asset)?;
        reserve.s_token_address.require_auth();
        Self::require_not_paused(&env)?;
        // TODO
        let balance_from_after = balance_from_before
            .checked_sub(amount)
            .ok_or(Error::InvalidAmount)?;

        Self::require_good_position(
            &env,
            from,
            Some((reserve.s_token_address, balance_from_after)),
            None,
            true,
        )?;

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

        let reserve = get_actual_reserve_data(&env, asset.clone())?;

        let s_token = STokenClient::new(&env, &reserve.s_token_address);
        let who_balance = s_token.balance(&who);
        let amount_to_withdraw = if amount == i128::MAX {
            who_balance
        } else {
            amount
        };

        Self::validate_withdraw(&env, who.clone(), &reserve, amount_to_withdraw, who_balance);

        let mut user_config: UserConfiguration = read_user_config(&env, who.clone())?;

        if amount_to_withdraw == who_balance {
            user_config.set_using_as_collateral(&env, reserve.get_id(), false);
            write_user_config(&env, who.clone(), &user_config);
            event::reserve_used_as_collateral_disabled(&env, who.clone(), asset.clone());
        }

        // amount_to_burn = amount_to_withdraw / liquidity_index
        let amount_to_burn = Self::get_collateral_coeff(&env, &reserve)?
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
    /// - Panics with `MustNotBeInCollateralAsset` if there is a collateral in borrowing asset.
    ///
    fn borrow(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error> {
        who.require_auth();
        Self::require_not_paused(&env)?;

        let reserve = get_actual_reserve_data(&env, asset.clone())?;
        let user_config = read_user_config(&env, who.clone())?;

        let s_token = STokenClient::new(&env, &reserve.s_token_address);
        let s_token_balance = s_token.balance(&who);

        Self::validate_borrow(
            &env,
            who.clone(),
            &asset,
            &reserve,
            &user_config,
            amount,
            s_token_balance,
        )?;

        let debt_token = DebtTokenClient::new(&env, &reserve.debt_token_address);
        let is_first_borrowing = debt_token.balance(&who) == 0;

        if is_first_borrowing {
            let mut user_config = user_config;
            user_config.set_borrowing(&env, reserve.get_id(), true);
            write_user_config(&env, who.clone(), &user_config);
        }

        debt_token.mint(&who, &amount);
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
        asset: &Address,
        amount: i128,
        reserve: &ReserveData,
    ) -> Result<bool, Error> {
        if amount == 0 {
            return Ok(false);
        }

        let collat_coeff = Self::get_collateral_coeff(env, reserve)?;
        let underlying_asset = token::Client::new(env, asset);
        let s_token = STokenClient::new(env, &reserve.s_token_address);
        let is_first_deposit = s_token.balance(who) == 0;

        let amount_to_mint = collat_coeff
            .recip_mul_int(amount)
            .ok_or(Error::MathOverflowError)?;

        underlying_asset.transfer(who, &reserve.s_token_address, &amount);
        s_token.mint(who, &amount_to_mint);

        event::deposit(env, who.clone(), asset.clone(), amount);

        Ok(is_first_deposit)
    }

    /// Returns (i128: the remaining amount after repayment, bool: the flag indicating the debt is fully repayed)
    fn do_repay(
        env: &Env,
        who: &Address,
        asset: &Address,
        amount: i128,
        reserve: &ReserveData,
    ) -> Result<(i128, bool), Error> {
        let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
        let asset_debt = debt_token.balance(who);

        if asset_debt == 0 {
            return Ok((amount, false));
        }

        let underlying_asset = token::Client::new(env, asset);
        let debt_coeff = Self::get_debt_coeff(env, reserve)?;

        let compounded_debt = debt_coeff
            .mul_int(asset_debt)
            .ok_or(Error::MathOverflowError)?;

        let payback_amount = amount.min(compounded_debt);

        let payback_debt = if payback_amount == compounded_debt {
            asset_debt
        } else {
            debt_coeff
                .recip_mul_int(payback_amount)
                .ok_or(Error::MathOverflowError)?
        };

        underlying_asset.transfer(who, &reserve.s_token_address, &payback_amount);
        debt_token.burn(who, &payback_debt);

        event::repay(env, who.clone(), asset.clone(), amount);

        let remaning_amount = amount
            .checked_sub(payback_amount)
            .ok_or(Error::MathOverflowError)?;

        let remaning_amount = if remaning_amount > 0 {
            remaning_amount
        } else {
            0
        };

        let is_repayed = compounded_debt == payback_amount;

        Ok((remaning_amount, is_repayed))
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
        amount: i128,
        balance: i128,
    ) {
        assert_with_error!(env, amount > 0, Error::InvalidAmount);
        let flags = reserve.configuration.get_flags();
        assert_with_error!(env, flags.is_active, Error::NoActiveReserve);
        assert_with_error!(env, amount <= balance, Error::NotEnoughAvailableUserBalance);

        match Self::is_good_position(env, who, None, None, true) {
            Ok(good_position) => assert_with_error!(env, good_position, Error::BadPosition),
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
        s_token_balance: i128,
    ) -> Result<(), Error> {
        assert_with_error!(env, s_token_balance == 0, Error::MustNotBeInCollateralAsset);

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
        let account_data = Self::calc_account_data(env, who.clone(), None, user_config, reserves)?;

        assert_with_error!(
            env,
            account_data.npv >= amount_in_xlm,
            Error::CollateralNotCoverNewBorrow
        );

        //TODO: complete validation after rate implementation
        Self::require_good_position(env, who, None, None, true)?;

        Ok(())
    }

    fn calc_account_data(
        env: &Env,
        who: Address,
        mb_who_balance: Option<(Address, i128)>,
        user_config: &UserConfiguration,
        reserves: &Vec<Address>,
    ) -> Result<AccountData, Error> {
        if user_config.is_empty() {
            return Ok(AccountData {
                collateral: 0,
                debt: 0,
                npv: 0,
            });
        }

        let mut total_collateral_in_xlm: i128 = 0;
        let mut total_debt_in_xlm: i128 = 0;
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
            }
        }

        let npv = total_collateral_in_xlm
            .checked_sub(total_debt_in_xlm)
            .ok_or(Error::CalcAccountDataMathError)?;

        Ok(AccountData {
            collateral: total_collateral_in_xlm,
            debt: total_debt_in_xlm,
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

    /// Returns collateral_accrued_rate corrected for current time
    fn get_collateral_coeff(env: &Env, reserve: &ReserveData) -> Result<FixedI128, Error> {
        let current_time = env.ledger().timestamp();
        let elapsed_time = current_time
            .checked_sub(reserve.last_update_timestamp)
            .ok_or(Error::CollateralCoeffMathError)?;
        let prev_ar = FixedI128::from_inner(reserve.collat_accrued_rate);
        if elapsed_time == 0 {
            Ok(prev_ar)
        } else {
            let lend_ir = FixedI128::from_inner(reserve.lend_ir);
            calc_accrued_rate_coeff(prev_ar, lend_ir, elapsed_time)
                .ok_or(Error::CollateralCoeffMathError)
        }
    }

    /// Returns debt_accrued_rate corrected for current time
    fn get_debt_coeff(env: &Env, reserve: &ReserveData) -> Result<FixedI128, Error> {
        let current_time = env.ledger().timestamp();
        let elapsed_time = current_time
            .checked_sub(reserve.last_update_timestamp)
            .ok_or(Error::DebtCoeffMathError)?;
        let prev_ar = FixedI128::from_inner(reserve.debt_accrued_rate);
        if elapsed_time == 0 {
            Ok(prev_ar)
        } else {
            let debt_ir = FixedI128::from_inner(reserve.debt_ir);
            calc_accrued_rate_coeff(prev_ar, debt_ir, elapsed_time).ok_or(Error::DebtCoeffMathError)
        }
    }

    fn require_not_paused(env: &Env) -> Result<(), Error> {
        if paused(env) {
            return Err(Error::Paused);
        }

        Ok(())
    }

    fn is_good_position(
        env: &Env,
        who: Address,
        mb_who_balance: Option<(Address, i128)>,
        mb_account_data: Option<AccountData>,
        is_good: bool,
    ) -> Result<bool, Error> {
        let _account_data = if let Some(account_data) = mb_account_data {
            account_data
        } else {
            let user_config = read_user_config(env, who.clone())?;
            let reserves = read_reserves(env);
            Self::calc_account_data(env, who, mb_who_balance, &user_config, &reserves)?
        };

        Ok(is_good)
    }

    fn require_good_position(
        env: &Env,
        who: Address,
        mb_who_balance: Option<(Address, i128)>,
        mb_account_data: Option<AccountData>,
        is_good: bool,
    ) -> Result<(), Error> {
        let is_good_position =
            Self::is_good_position(env, who, mb_who_balance, mb_account_data, is_good)?;
        if !is_good_position {
            return Err(Error::BadPosition);
        }

        Ok(())
    }
}

/// Returns reserve data with updated accrued coeffiсients
pub fn get_actual_reserve_data(env: &Env, asset: Address) -> Result<ReserveData, Error> {
    let reserve = read_reserve(env, asset.clone())?;
    let current_time = env.ledger().timestamp();
    let elapsed_time = current_time
        .checked_sub(reserve.last_update_timestamp)
        .ok_or(Error::AccruedRateMathError)?;
    if elapsed_time == 0 {
        return Ok(reserve);
    }

    let s_token = STokenClient::new(env, &reserve.s_token_address);
    let total_collateral = s_token.total_supply();

    let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
    let total_debt = debt_token.total_supply();
    let ir_params = read_ir_params(env)?;
    let accrued_rates = calc_accrued_rates(
        total_collateral,
        total_debt,
        elapsed_time,
        ir_params,
        &reserve,
    )
    .ok_or(Error::AccruedRateMathError)?;

    let mut reserve = reserve;
    reserve.collat_accrued_rate = accrued_rates.collat_accrued_rate.into_inner();
    reserve.debt_accrued_rate = accrued_rates.debt_accrued_rate.into_inner();
    reserve.debt_ir = accrued_rates.debt_ir.into_inner();
    reserve.lend_ir = accrued_rates.lend_ir.into_inner();
    reserve.last_update_timestamp = current_time;

    write_reserve(env, asset, &reserve);
    Ok(reserve)
}
