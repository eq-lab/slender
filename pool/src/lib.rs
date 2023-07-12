#![deny(warnings)]
#![no_std]
use crate::price_provider::PriceProvider;
use common::{percentage_math::*, rate_math::*, FixedPoint};
use debt_token_interface::DebtTokenClient;
use pool_interface::*;
use s_token_interface::STokenClient;
use soroban_sdk::{
    assert_with_error, contractimpl, panic_with_error, token, Address, BytesN, Env, Vec,
};

mod event;
mod price_provider;
mod storage;

use crate::storage::*;

#[allow(dead_code)] //TODO: remove after full implement validate_borrow
#[derive(Debug, Clone, Copy)]
struct AccountData {
    collateral: i128,
    debt: i128,
    ltv: i128,
    liq_threshold: i128,
    health_factor: i128,
    /// Net position value
    npv: i128,
}

//TODO: set right value for liquidation threshold
const HEALTH_FACTOR_LIQUIDATION_THRESHOLD: i128 = 1;

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
        Self::require_admin(&env)?;
        // require_contract(env, asset)?;
        if has_reserve(&env, asset.clone()) {
            panic_with_error!(&env, Error::ReserveAlreadyInitialized);
        }

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
    /// - asset The address of asset that should be set as collateral
    /// - config Collateral parameters
    ///
    /// # Panics
    ///
    /// - Panics with `InvalidReserveParams` when wrong collateral params provided.
    /// - Panics if the caller is not the admin.
    fn configure_as_collateral(
        env: Env,
        asset: Address,
        params: CollateralParamsInput,
    ) -> Result<(), Error> {
        Self::require_admin(&env)?;

        assert_with_error!(
            &env,
            params.discount <= PERCENTAGE_FACTOR,
            Error::InvalidReserveParams
        );

        //validation of the parameters: the LTV can
        //only be lower or equal than the liquidation threshold
        //(otherwise a loan against the asset would cause instantaneous liquidation)
        assert_with_error!(
            &env,
            params.ltv <= params.liq_threshold,
            Error::InvalidReserveParams
        );

        if params.liq_threshold != 0 {
            //liquidation bonus must be bigger than 100.00%, otherwise the liquidator would receive less
            //collateral than needed to cover the debt
            assert_with_error!(
                &env,
                params.liq_bonus > PERCENTAGE_FACTOR,
                Error::InvalidReserveParams
            );

            //if threshold * bonus is less than or equal to PERCENTAGE_FACTOR, it's guaranteed that at the moment
            //a loan is taken there is enough collateral available to cover the liquidation bonus
            assert_with_error!(
                env,
                params
                    .liq_threshold
                    .percent_mul(params.liq_bonus)
                    .ok_or(Error::MathOverflowError)?
                    <= PERCENTAGE_FACTOR as i128,
                Error::InvalidReserveParams
            );
        } else {
            assert_with_error!(&env, params.liq_bonus == 0, Error::InvalidReserveParams);

            //if the liquidation threshold is being set to 0,
            // the reserve is being disabled as collateral. To do so,
            //we need to ensure no liquidity is deposited
            Self::require_no_liquidity(&env, asset.clone())?;
        }

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

        let mut user_config: UserConfiguration = read_user_config(&env, who.clone())?;

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
    fn borrow(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error> {
        who.require_auth();

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

    #[cfg(any(test, feature = "testutils"))]
    fn set_liq_index(env: Env, asset: Address, value: i128) -> Result<(), Error> {
        let mut reserve_data = read_reserve(&env, asset.clone())?;
        reserve_data.liquidity_index = value;
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
        assert_with_error!(env, amount > 0, Error::InvalidAmount);
        let flags = reserve.configuration.get_flags();
        assert_with_error!(env, flags.is_active, Error::NoActiveReserve);
        assert_with_error!(env, !flags.is_frozen, Error::ReserveFrozen);
    }

    fn validate_withdraw(reserve: &ReserveData, env: &Env, amount: i128, balance: i128) {
        assert_with_error!(env, amount > 0, Error::InvalidAmount);
        let flags = reserve.configuration.get_flags();
        assert_with_error!(env, flags.is_active, Error::NoActiveReserve);
        assert_with_error!(env, amount <= balance, Error::NotEnoughAvailableUserBalance);

        //TODO: implement when rates exists
        //balance_decrease_allowed()
    }

    fn validate_borrow(
        env: &Env,
        who: Address,
        asset: &Address,
        reserve: &ReserveData,
        user_config: &UserConfiguration,
        amount: i128,
    ) -> Result<(), Error> {
        let (asset_price, denominator) = Self::get_asset_price(env, asset.clone())?;
        let amount_in_xlm = amount
            .fixed_mul_floor(asset_price, denominator)
            .ok_or(Error::ValidateBorrowMathError)?;

        assert_with_error!(env, amount > 0 && amount_in_xlm > 0, Error::InvalidAmount);
        let flags = reserve.configuration.get_flags();
        assert_with_error!(env, flags.is_active, Error::NoActiveReserve);
        assert_with_error!(env, !flags.is_frozen, Error::ReserveFrozen);
        assert_with_error!(env, flags.borrowing_enabled, Error::BorrowingNotEnabled);

        let reserves = &read_reserves(env);
        let account_data = Self::calc_account_data(env, who, user_config, reserves)?;

        assert_with_error!(env, account_data.collateral > 0, Error::CollateralIsZero);
        assert_with_error!(
            env,
            account_data.health_factor > HEALTH_FACTOR_LIQUIDATION_THRESHOLD,
            Error::HealthFactorLowerThanLiqThreshold
        );

        assert_with_error!(
            env,
            account_data.npv >= amount_in_xlm,
            Error::CollateralNotCoverNewBorrow
        );

        let amount_of_collateral_needed_xlm = account_data
            .debt
            .checked_add(amount_in_xlm)
            .ok_or(Error::ValidateBorrowMathError)?
            .percent_div(account_data.ltv)
            .ok_or(Error::ValidateBorrowMathError)?;

        assert_with_error!(
            env,
            amount_of_collateral_needed_xlm <= account_data.collateral,
            Error::CollateralNotCoverNewBorrow
        );

        //TODO: complete validation after rate implementation
        Ok(())
    }

    fn calc_account_data(
        env: &Env,
        who: Address,
        user_config: &UserConfiguration,
        reserves: &Vec<Address>,
    ) -> Result<AccountData, Error> {
        if user_config.is_empty() {
            return Ok(AccountData {
                collateral: 0,
                debt: 0,
                ltv: 0,
                liq_threshold: 0,
                health_factor: i128::MAX,
                npv: 0,
            });
        }

        let mut total_collateral_in_xlm: i128 = 0;
        let mut total_debt_in_xlm: i128 = 0;
        let mut avg_ltv: i128 = 0;
        let mut avg_liq_threshold: i128 = 0;
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

            let (reserve_price, price_denominator) =
                Self::get_asset_price(env, curr_reserve_asset.clone())?;

            if curr_reserve.configuration.liq_threshold != 0
                && user_config.is_using_as_collateral(env, i)
            {
                let coll_coeff = Self::get_collateral_coeff(env, &curr_reserve)?;

                // compounded balance of sToken
                let s_token =
                    s_token_interface::STokenClient::new(env, &curr_reserve.s_token_address);

                let compounded_balance = s_token
                    .balance(&who)
                    .mul_rate_floor(coll_coeff)
                    .ok_or(Error::CalcAccountDataMathError)?
                    .percent_mul(curr_reserve.configuration.discount)
                    .ok_or(Error::CalcAccountDataMathError)?;

                let liquidity_balance_in_xlm = compounded_balance
                    .fixed_mul_floor(reserve_price, price_denominator)
                    .ok_or(Error::CalcAccountDataMathError)?;

                total_collateral_in_xlm = total_collateral_in_xlm
                    .checked_add(liquidity_balance_in_xlm)
                    .ok_or(Error::CalcAccountDataMathError)?;

                avg_ltv = avg_ltv
                    .checked_add(
                        i128::from(curr_reserve.configuration.ltv)
                            .checked_mul(liquidity_balance_in_xlm)
                            .ok_or(Error::CalcAccountDataMathError)?,
                    )
                    .ok_or(Error::CalcAccountDataMathError)?;

                avg_liq_threshold = avg_liq_threshold
                    .checked_add(
                        i128::from(curr_reserve.configuration.liq_threshold)
                            .checked_mul(liquidity_balance_in_xlm)
                            .ok_or(Error::CalcAccountDataMathError)?,
                    )
                    .ok_or(Error::CalcAccountDataMathError)?;
            }

            if user_config.is_borrowing(env, i) {
                let debt_coeff = Self::get_debt_coeff(env, &curr_reserve)?;

                let debt_token = token::Client::new(env, &curr_reserve.debt_token_address);
                let compounded_balance = debt_token
                    .balance(&who)
                    .mul_rate_floor(debt_coeff)
                    .ok_or(Error::CalcAccountDataMathError)?;

                let debt_balance_in_xlm = compounded_balance
                    .fixed_div_floor(reserve_price, price_denominator)
                    .ok_or(Error::CalcAccountDataMathError)?;

                total_debt_in_xlm = total_debt_in_xlm
                    .checked_add(debt_balance_in_xlm)
                    .ok_or(Error::CalcAccountDataMathError)?;
            }
        }

        avg_ltv = avg_ltv.checked_div(total_collateral_in_xlm).unwrap_or(0);
        avg_liq_threshold = avg_liq_threshold
            .checked_div(total_collateral_in_xlm)
            .unwrap_or(0);

        let npv = total_collateral_in_xlm
            .checked_sub(total_debt_in_xlm)
            .ok_or(Error::CalcAccountDataMathError)?;

        Ok(AccountData {
            collateral: total_collateral_in_xlm,
            debt: total_debt_in_xlm,
            ltv: avg_ltv,
            liq_threshold: avg_liq_threshold,
            health_factor: Self::calc_health_factor(
                total_collateral_in_xlm,
                total_debt_in_xlm,
                avg_liq_threshold,
            )?,
            npv,
        })
    }

    /// Returns price of asset expressed in XLM token and denominator 10^decimals
    fn get_asset_price(env: &Env, asset: Address) -> Result<(i128, i128), Error> {
        let price_feed = read_price_feed(env, asset.clone())?;
        let provider = PriceProvider::new(env, &price_feed);
        provider
            .get_price(&asset)
            .ok_or(Error::NoPriceForAsset)
            .map(|price_data| {
                Ok((
                    price_data.price,
                    10_i128
                        .checked_pow(price_data.decimals)
                        .ok_or(Error::PriceMathOverflow)?,
                ))
            })?
    }

    fn calc_health_factor(
        total_collateral: i128,
        total_debt: i128,
        liquidation_threshold: i128,
    ) -> Result<i128, Error> {
        if total_debt == 0 {
            return Ok(i128::MAX);
        }

        total_collateral
            .percent_mul(liquidation_threshold)
            .ok_or(Error::MathOverflowError)?
            .checked_div(total_debt)
            .ok_or(Error::MathOverflowError)
    }

    fn get_collateral_coeff(_env: &Env, _reserve: &ReserveData) -> Result<i128, Error> {
        //TODO: implement rate
        Ok(RATE_DENOMINATOR)
    }

    fn get_debt_coeff(_env: &Env, _reserve: &ReserveData) -> Result<i128, Error> {
        //TODO: implement accrued
        Ok(RATE_DENOMINATOR)
    }

    fn require_no_liquidity(env: &Env, asset: Address) -> Result<(), Error> {
        let reserve = read_reserve(env, asset.clone())?;
        let token = token::Client::new(env, &asset);

        assert_with_error!(
            env,
            token.balance(&reserve.s_token_address) == 0 && reserve.current_liquidity_rate == 0,
            Error::ReserveLiquidityNotZero
        );

        Ok(())
    }
}

#[cfg(test)]
mod test;
