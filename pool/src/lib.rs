#![deny(warnings)]
#![no_std]

use crate::price_provider::PriceProvider;
use common::{FixedI128, PERCENTAGE_FACTOR};
use debt_token_interface::DebtTokenClient;
use pool_interface::*;
use rate::{calc_accrued_rates, calc_next_accrued_rate};
use s_token_interface::STokenClient;
use soroban_sdk::{
    assert_with_error, contractimpl, contracttype, panic_with_error, token,
    token::Client as TokenClient, unwrap::UnwrapOptimized, vec, Address, BytesN, Env, Map, Vec,
};

mod event;
mod price_provider;
mod rate;
mod storage;
#[cfg(test)]
mod test;

use crate::storage::*;

#[allow(dead_code)] //TODO: remove after full implement validate_borrow
#[derive(Debug, Clone)]
struct AccountData {
    /// Total collateral expresed in XLM
    discounted_collateral: i128,
    /// Total debt expressed in XLM
    debt: i128,
    /// Net position value in XLM
    npv: i128,
    /// Liquidation data
    liquidation: Option<LiquidationData>,
}

impl AccountData {
    pub fn is_good_position(&self) -> bool {
        self.npv > 0
    }

    pub fn get_position(&self) -> AccountPosition {
        AccountPosition {
            discounted_collateral: self.discounted_collateral,
            debt: self.debt,
            npv: self.npv,
        }
    }
}

#[derive(Debug, Clone)]
struct LiquidationData {
    total_debt_with_penalty_in_xlm: i128,
    /// reserve data, compounded debt, debtToken balance
    debt_to_cover: Vec<(ReserveData, i128, i128)>,
    /// reserve data, stoken balance, collateral asset price, AR coefficient
    collateral_to_receive: Vec<(ReserveData, i128, i128, i128)>,
}

impl LiquidationData {
    fn default(env: &Env) -> Self {
        Self {
            total_debt_with_penalty_in_xlm: Default::default(),
            debt_to_cover: vec![env],
            collateral_to_receive: vec![env],
        }
    }
}

#[derive(Debug, Clone)]
#[contracttype]
struct AssetBalance {
    asset: Address,
    balance: i128,
}

impl AssetBalance {
    fn new(asset: Address, balance: i128) -> Self {
        Self { asset, balance }
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
    /// - treasury - The address of the treasury contract.
    /// - ir_params - The interest rate parameters to set.
    ///
    /// # Panics
    ///
    /// Panics with `AlreadyInitialized` if the admin key already exists in storage.
    ///
    fn initialize(
        env: Env,
        admin: Address,
        treasury: Address,
        ir_params: IRParams,
    ) -> Result<(), Error> {
        if has_admin(&env) {
            panic_with_error!(&env, Error::AlreadyInitialized);
        }
        Self::require_valid_ir_params(&env, &ir_params);

        write_admin(&env, admin.clone());
        write_treasury(&env, &treasury);
        write_ir_params(&env, &ir_params);

        event::initialized(&env, admin, treasury, ir_params);

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
    /// - Panics with `MustBeLtePercentageFactor` if util_cap or discount is invalid.
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

    /// Returns collateral coefficient corrected on current time expressed as inner value of FixedI128
    ///
    /// # Arguments
    ///
    /// - asset - The address of underlying asset
    fn collat_coeff(env: Env, asset: Address) -> Result<i128, Error> {
        let reserve = read_reserve(&env, asset.clone())?;
        Self::get_collat_coeff(&env, &asset, &reserve).map(|fixed| fixed.into_inner())
    }

    /// Returns debt coefficient corrected on current time expressed as inner value of FixedI128.
    /// The same as borrower accrued rate
    ///
    /// # Arguments
    ///
    /// - asset - The address of underlying asset
    fn debt_coeff(env: Env, asset: Address) -> Result<i128, Error> {
        let reserve = read_reserve(&env, asset)?;
        Self::get_debt_coeff(&env, &reserve).map(|fixed| fixed.into_inner())
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
    /// - amount - The amount to be repayed/deposited. Use i128::MAX to repay the maximum available amount.
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
    fn deposit(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error> {
        who.require_auth();
        Self::require_not_paused(&env);

        let reserve = get_actual_reserve_data(&env, asset.clone())?;
        Self::validate_deposit(&env, &reserve, amount);

        let (remaining_amount, is_repayed) =
            Self::do_repay(&env, &who, &asset, amount, None, &reserve)?;
        let is_first_deposit = Self::do_deposit(&env, &who, &asset, remaining_amount, &reserve)?;

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
    ///
    /// # Panics
    ///
    /// Panics if the caller is not the sToken contract.
    ///
    #[allow(clippy::too_many_arguments)]
    fn finalize_transfer(
        env: Env,
        asset: Address,
        from: Address,
        to: Address,
        amount: i128,
        balance_from_before: i128,
        balance_to_before: i128,
        s_token_supply: i128,
    ) -> Result<(), Error> {
        // TODO: maybe check with callstack?

        let reserve = read_reserve(&env, asset.clone())?;
        let s_token_address = (reserve.clone()).s_token_address;
        s_token_address.require_auth();

        Self::require_zero_debt(&env, to.clone(), reserve.debt_token_address.clone())?;

        // update reserve
        let reserve = recalculate_reserve_data(&env, asset.clone(), reserve, s_token_supply)?;

        Self::require_not_paused(&env);
        let balance_from_after = balance_from_before
            .checked_sub(amount)
            .ok_or(Error::InvalidAmount)?;

        let mut from_config = read_user_config(&env, from.clone())?;
        let reserves = read_reserves(&env);
        let account_data = Self::calc_account_data(
            &env,
            from.clone(),
            Some(AssetBalance::new(s_token_address, balance_from_after)),
            &from_config,
            &reserves,
            false,
        )?;
        Self::require_good_position(account_data)?;

        if from != to {
            let reserve_id = reserve.get_id();
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
        Self::require_not_paused(&env);

        let reserve = get_actual_reserve_data(&env, asset.clone())?;

        let s_token = STokenClient::new(&env, &reserve.s_token_address);
        let s_token_balance = s_token.balance(&who);

        let collat_coeff = Self::get_collat_coeff(&env, &asset, &reserve)?;
        let underlying_balance = collat_coeff
            .mul_int(s_token_balance)
            .ok_or(Error::MathOverflowError)?;

        let (underlying_to_withdraw, s_token_to_burn) = if amount == i128::MAX {
            (underlying_balance, s_token_balance)
        } else {
            // s_token_to_burn = underlying_to_withdraw / collat_coeff
            let s_token_to_burn = collat_coeff
                .recip_mul_int(amount)
                .ok_or(Error::MathOverflowError)?;
            (amount, s_token_to_burn)
        };

        let mut user_config: UserConfiguration = read_user_config(&env, who.clone())?;
        let s_token_balance_after = s_token_balance
            .checked_sub(s_token_to_burn)
            .ok_or(Error::InvalidAmount)?;
        let s_token_address = s_token.address.clone();
        Self::validate_withdraw(
            &env,
            who.clone(),
            &reserve,
            &user_config,
            AssetBalance::new(s_token_address, s_token_balance_after),
            underlying_to_withdraw,
            underlying_balance,
        )?;

        if underlying_to_withdraw == underlying_balance {
            user_config.set_using_as_collateral(&env, reserve.get_id(), false);
            write_user_config(&env, who.clone(), &user_config);
            event::reserve_used_as_collateral_disabled(&env, who.clone(), asset.clone());
        }

        s_token.burn(&who, &s_token_to_burn, &underlying_to_withdraw, &to);

        event::withdraw(&env, who, asset, to, underlying_to_withdraw);
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
    /// - Panics with `UtilizationCapExceeded` if utilization after borrow is above the limit.
    ///
    fn borrow(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error> {
        who.require_auth();
        Self::require_not_paused(&env);

        let reserve = get_actual_reserve_data(&env, asset.clone())?;
        let user_config = read_user_config(&env, who.clone())?;

        let s_token = STokenClient::new(&env, &reserve.s_token_address);
        let debt_token = DebtTokenClient::new(&env, &reserve.debt_token_address);

        Self::validate_borrow(
            &env,
            who.clone(),
            &asset,
            &reserve,
            &user_config,
            &s_token,
            &debt_token,
            amount,
        )?;

        let debt_coeff = Self::get_debt_coeff(&env, &reserve)?;
        let amount_of_debt_token = debt_coeff
            .recip_mul_int(amount)
            .ok_or(Error::MathOverflowError)?;

        let debt_token = DebtTokenClient::new(&env, &reserve.debt_token_address);

        let is_first_borrowing = debt_token.balance(&who) == 0;

        if is_first_borrowing {
            let mut user_config = user_config;
            user_config.set_borrowing(&env, reserve.get_id(), true);
            write_user_config(&env, who.clone(), &user_config);
        }

        debt_token.mint(&who, &amount_of_debt_token);
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

    /// Retrieves the address of the treasury.
    ///
    /// # Returns
    ///
    /// The address of the treasury.
    ///
    fn treasury(e: Env) -> Address {
        read_treasury(&e)
    }

    /// Retrieves the account position info.
    ///
    /// # Arguments
    /// - who The address for which the position info is getting
    ///
    /// # Panics
    /// - Panics if position can't be calculated
    ///
    /// # Returns
    ///
    /// Returns the position info.
    ///
    fn get_account_position(env: Env, who: Address) -> Result<AccountPosition, Error> {
        let account_data = Self::calc_account_data(
            &env,
            who.clone(),
            None,
            &read_user_config(&env, who)?,
            &read_reserves(&env),
            false,
        )?;
        Ok(account_data.get_position())
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
        Self::require_not_paused(&env);
        let reserves = read_reserves(&env);
        let mut user_config = read_user_config(&env, who.clone())?;
        let account_data =
            Self::calc_account_data(&env, who.clone(), None, &user_config, &reserves, true)?;
        if account_data.is_good_position() {
            return Err(Error::GoodPosition);
        }

        // let liquidation_debt = account_data
        //     .debt_with_penalty
        //     .expect("pool: liquidation flag in calc_account_data");

        Self::do_liquidate(
            &env,
            liquidator,
            who.clone(),
            &mut user_config,
            account_data.clone(),
            receive_stoken,
        )?;
        event::liquidation(
            &env,
            who,
            account_data.debt,
            account_data
                .liquidation
                .unwrap_optimized()
                .total_debt_with_penalty_in_xlm,
        );

        Ok(())
    }

    #[cfg(any(test, feature = "testutils"))]
    fn set_accrued_rates(
        env: Env,
        asset: Address,
        mb_lender_accrued_rate: Option<i128>,
        mb_borrower_accrued_rate: Option<i128>,
    ) -> Result<(), Error> {
        let mut reserve_data = read_reserve(&env, asset.clone())?;

        if let Some(value) = mb_lender_accrued_rate {
            reserve_data.lender_accrued_rate = value;
        }

        if let Some(value) = mb_borrower_accrued_rate {
            reserve_data.borrower_accrued_rate = value;
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
        Self::require_lte_percentage_factor(env, params.util_cap);
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

    /// Check that balance + deposit + debt * ar_lender <= reserve.configuration.liq_cap
    fn require_liq_cap_not_exceeded(
        env: &Env,
        reserve: &ReserveData,
        balance: i128,
        deposit_amount: i128,
    ) -> Result<(), Error> {
        let balance_after_deposit = {
            let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
            let debt_supply = debt_token.total_supply();

            FixedI128::from_inner(reserve.lender_accrued_rate)
                .mul_int(debt_supply)
                .ok_or(Error::MathOverflowError)?
                .checked_add(deposit_amount)
                .ok_or(Error::MathOverflowError)?
                .checked_add(balance)
                .ok_or(Error::MathOverflowError)?
        };

        assert_with_error!(
            env,
            balance_after_deposit <= reserve.configuration.liq_cap,
            Error::LiqCapExceeded
        );

        Ok(())
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

        let underlying_asset = token::Client::new(env, asset);

        //TODO: use own aggregate instead of token.balance
        let balance = underlying_asset.balance(&reserve.s_token_address);

        Self::require_liq_cap_not_exceeded(env, reserve, balance, amount)?;

        let s_token = STokenClient::new(env, &reserve.s_token_address);
        let is_first_deposit = s_token.balance(who) == 0;

        let collat_coeff = Self::get_collat_coeff(env, asset, reserve)?;
        let amount_to_mint = collat_coeff
            .recip_mul_int(amount)
            .ok_or(Error::MathOverflowError)?;

        underlying_asset.transfer(who, &reserve.s_token_address, &amount);

        let s_token = STokenClient::new(env, &reserve.s_token_address);
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
        mb_debt_amount: Option<i128>,
        reserve: &ReserveData,
    ) -> Result<(i128, bool), Error> {
        let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
        let borrower_total_debt: i128 = if let Some(mb_debt_amount) = mb_debt_amount {
            mb_debt_amount
        } else {
            debt_token.balance(who)
        };

        if borrower_total_debt == 0 {
            return Ok((amount, false));
        }

        let debt_coeff = Self::get_debt_coeff(env, reserve)?;
        let collat_coeff = Self::get_collat_coeff(env, asset, reserve)?;

        let borrower_actual_debt = debt_coeff
            .mul_int(borrower_total_debt)
            .ok_or(Error::MathOverflowError)?;

        let (borrower_payback_amount, borrower_debt_to_burn, is_repayed) =
            if amount >= borrower_actual_debt {
                // To avoid dust in debt_token borrower balance in case of full repayment
                (borrower_actual_debt, borrower_total_debt, true)
            } else {
                let borrower_debt_to_burn = debt_coeff
                    .recip_mul_int(amount)
                    .ok_or(Error::MathOverflowError)?;

                (amount, borrower_debt_to_burn, false)
            };

        let lender_part = collat_coeff
            .mul_int(borrower_debt_to_burn)
            .ok_or(Error::MathOverflowError)?;
        let treasury_part = borrower_payback_amount
            .checked_sub(lender_part)
            .ok_or(Error::MathOverflowError)?;

        let treasury_address = read_treasury(env);
        let underlying_asset = token::Client::new(env, asset);
        underlying_asset.transfer(who, &reserve.s_token_address, &lender_part);
        underlying_asset.transfer(who, &treasury_address, &treasury_part);
        debt_token.burn(who, &borrower_debt_to_burn);

        event::repay(env, who.clone(), asset.clone(), amount);

        let remaning_amount = if amount != i128::MAX && amount > borrower_actual_debt {
            amount
                .checked_sub(borrower_actual_debt)
                .ok_or(Error::MathOverflowError)?
        } else {
            0
        };

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
        user_config: &UserConfiguration,
        s_token_after: AssetBalance,
        amount: i128,
        balance: i128,
    ) -> Result<(), Error> {
        assert_with_error!(env, amount > 0, Error::InvalidAmount);
        let flags = reserve.configuration.get_flags();
        assert_with_error!(env, flags.is_active, Error::NoActiveReserve);
        assert_with_error!(env, amount <= balance, Error::NotEnoughAvailableUserBalance);

        let reserves = read_reserves(env);
        if user_config.is_borrowing_any() {
            let account_data = Self::calc_account_data(
                env,
                who,
                Some(s_token_after),
                user_config,
                &reserves,
                false,
            )?;
            Self::require_good_position(account_data)?;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn validate_borrow(
        env: &Env,
        who: Address,
        asset: &Address,
        reserve: &ReserveData,
        user_config: &UserConfiguration,
        s_token: &STokenClient,
        debt_token: &DebtTokenClient,
        amount_to_borrow: i128,
    ) -> Result<(), Error> {
        let s_token_balance = s_token.balance(&who);
        assert_with_error!(env, s_token_balance == 0, Error::MustNotBeInCollateralAsset);

        let total_debt_after = debt_token
            .total_supply()
            .checked_add(amount_to_borrow)
            .ok_or(Error::ValidateBorrowMathError)?;
        let total_collat = s_token.total_supply();
        let utilization = FixedI128::from_rational(total_debt_after, total_collat)
            .ok_or(Error::ValidateBorrowMathError)?;
        let util_cap = FixedI128::from_percentage(reserve.configuration.util_cap)
            .ok_or(Error::ValidateBorrowMathError)?;
        assert_with_error!(env, utilization <= util_cap, Error::UtilizationCapExceeded);

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
        let account_data = Self::calc_account_data(env, who, None, user_config, reserves, false)?;

        assert_with_error!(
            env,
            account_data.npv >= amount_in_xlm,
            Error::CollateralNotCoverNewBorrow
        );

        Self::require_good_position(account_data)?;

        Ok(())
    }

    fn calc_account_data(
        env: &Env,
        who: Address,
        mb_who_balance: Option<AssetBalance>,
        user_config: &UserConfiguration,
        reserves: &Vec<Address>,
        liquidation: bool,
    ) -> Result<AccountData, Error> {
        if user_config.is_empty() {
            return Ok(AccountData {
                discounted_collateral: 0,
                debt: 0,
                liquidation: liquidation.then_some(LiquidationData::default(env)),
                npv: 0,
            });
        }

        let mut total_discounted_collateral_in_xlm: i128 = 0;
        let mut total_debt_in_xlm: i128 = 0;
        let mut total_debt_with_penalty_in_xlm: i128 = 0;
        let mut debt_to_cover = Vec::new(env);
        let mut sorted_collateral_to_receive = Map::new(env);
        let reserves_len =
            u8::try_from(reserves.len()).map_err(|_| Error::ReservesMaxCapacityExceeded)?;

        // calc collateral and debt expressed in XLM token
        for i in 0..reserves_len {
            if !user_config.is_using_as_collateral_or_borrowing(env, i) {
                continue;
            }

            let curr_reserve_asset = reserves.get_unchecked(i.into()).unwrap_optimized();
            let curr_reserve = read_reserve(env, curr_reserve_asset.clone())?;

            if !curr_reserve.configuration.is_active && liquidation {
                return Err(Error::NoActiveReserve);
            }

            let reserve_price = Self::get_asset_price(env, curr_reserve_asset.clone())?;

            if user_config.is_using_as_collateral(env, i) {
                let collat_coeff = Self::get_collat_coeff(env, &curr_reserve_asset, &curr_reserve)?;

                let who_balance: i128 = match &mb_who_balance {
                    Some(AssetBalance { asset, balance })
                        if *asset == curr_reserve.s_token_address =>
                    {
                        *balance
                    }
                    _ => STokenClient::new(env, &curr_reserve.s_token_address).balance(&who),
                };

                let discount = FixedI128::from_percentage(curr_reserve.configuration.discount)
                    .ok_or(Error::CalcAccountDataMathError)?;

                let compounded_balance = collat_coeff
                    .mul_int(who_balance)
                    .ok_or(Error::CalcAccountDataMathError)?;

                let compounded_balance_in_xlm = reserve_price
                    .mul_int(compounded_balance)
                    .ok_or(Error::CalcAccountDataMathError)?;

                let discounted_balance_in_xlm = discount
                    .mul_int(compounded_balance_in_xlm)
                    .ok_or(Error::CalcAccountDataMathError)?;

                total_discounted_collateral_in_xlm = total_discounted_collateral_in_xlm
                    .checked_add(discounted_balance_in_xlm)
                    .ok_or(Error::CalcAccountDataMathError)?;

                if liquidation {
                    let curr_discount = curr_reserve.configuration.discount;
                    let mut collateral_to_receive = sorted_collateral_to_receive
                        .get(curr_discount)
                        .unwrap_or(Ok(Vec::new(env)))
                        .expect("sorted");
                    collateral_to_receive.push_back((
                        curr_reserve,
                        who_balance,
                        reserve_price.into_inner(),
                        collat_coeff.into_inner(),
                    ));
                    sorted_collateral_to_receive.set(curr_discount, collateral_to_receive);
                }
            } else if user_config.is_borrowing(env, i) {
                let debt_coeff = Self::get_debt_coeff(env, &curr_reserve)?;

                let debt_token = token::Client::new(env, &curr_reserve.debt_token_address);
                let debt_token_balance = debt_token.balance(&who);
                let compounded_balance = debt_coeff
                    .mul_int(debt_token_balance)
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
                    total_debt_with_penalty_in_xlm = total_debt_with_penalty_in_xlm
                        .checked_add(liquidation_debt)
                        .ok_or(Error::CalcAccountDataMathError)?;

                    debt_to_cover.push_back((curr_reserve, compounded_balance, debt_token_balance));
                }
            }
        }

        let npv = total_discounted_collateral_in_xlm
            .checked_sub(total_debt_in_xlm)
            .ok_or(Error::CalcAccountDataMathError)?;

        let liquidation_data = || -> LiquidationData {
            let mut collateral_to_receive = vec![env];
            let sorted = sorted_collateral_to_receive.values();
            for v in sorted {
                for c in v.unwrap_optimized() {
                    collateral_to_receive.push_back(c.unwrap_optimized());
                }
            }

            LiquidationData {
                total_debt_with_penalty_in_xlm,
                debt_to_cover,
                collateral_to_receive,
            }
        };

        Ok(AccountData {
            discounted_collateral: total_discounted_collateral_in_xlm,
            debt: total_debt_in_xlm,
            liquidation: liquidation.then_some(liquidation_data()),
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
                FixedI128::from_rational(price_data.price, 10i128.pow(price_data.decimals))
                    .ok_or(Error::AssetPriceMathError)
            })?
    }

    /// Returns lender accrued rate corrected for the current time
    fn get_actual_lender_accrued_rate(
        env: &Env,
        reserve: &ReserveData,
    ) -> Result<FixedI128, Error> {
        let current_time = env.ledger().timestamp();
        let elapsed_time = current_time
            .checked_sub(reserve.last_update_timestamp)
            .ok_or(Error::CollateralCoeffMathError)?;
        let prev_ar = FixedI128::from_inner(reserve.lender_accrued_rate);
        if elapsed_time == 0 {
            Ok(prev_ar)
        } else {
            let lender_ir = FixedI128::from_inner(reserve.lender_ir);
            calc_next_accrued_rate(prev_ar, lender_ir, elapsed_time)
                .ok_or(Error::CollateralCoeffMathError)
        }
    }

    /// Returns collateral coefficient
    /// collateral_coeff = [underlying_balance + lender_accrued_rate * total_debt_token]/total_stoken
    fn get_collat_coeff(
        env: &Env,
        asset: &Address,
        reserve: &ReserveData,
    ) -> Result<FixedI128, Error> {
        let s_token = STokenClient::new(env, &reserve.s_token_address);
        let s_token_supply = s_token.total_supply();

        if s_token_supply == 0 {
            return Ok(FixedI128::ONE);
        }

        let collat_ar = Self::get_actual_lender_accrued_rate(env, reserve)?;

        //TODO: use own aggregate instead of balance()
        let balance = TokenClient::new(env, asset).balance(&reserve.s_token_address);
        let debt_token_supply =
            DebtTokenClient::new(env, &reserve.debt_token_address).total_supply();

        FixedI128::from_rational(
            balance
                .checked_add(
                    collat_ar
                        .mul_int(debt_token_supply)
                        .ok_or(Error::CollateralCoeffMathError)?,
                )
                .ok_or(Error::CollateralCoeffMathError)?,
            s_token_supply,
        )
        .ok_or(Error::CollateralCoeffMathError)
    }

    /// Returns borrower accrued rate corrected for the current time
    fn get_actual_borrower_accrued_rate(
        env: &Env,
        reserve: &ReserveData,
    ) -> Result<FixedI128, Error> {
        let current_time = env.ledger().timestamp();
        let elapsed_time = current_time
            .checked_sub(reserve.last_update_timestamp)
            .ok_or(Error::DebtCoeffMathError)?;
        let prev_ar = FixedI128::from_inner(reserve.borrower_accrued_rate);
        if elapsed_time == 0 {
            Ok(prev_ar)
        } else {
            let debt_ir = FixedI128::from_inner(reserve.borrower_ir);
            calc_next_accrued_rate(prev_ar, debt_ir, elapsed_time).ok_or(Error::DebtCoeffMathError)
        }
    }

    /// The same as borrower accrued rate
    fn get_debt_coeff(env: &Env, reserve: &ReserveData) -> Result<FixedI128, Error> {
        Self::get_actual_borrower_accrued_rate(env, reserve)
    }

    fn require_not_paused(env: &Env) {
        assert_with_error!(env, !paused(env), Error::Paused);
    }

    fn require_zero_debt(env: &Env, recipient: Address, debt_token_address: Address) {
        let debt_token = DebtTokenClient::new(env, &debt_token_address);
        assert_with_error!(
            env,
            debt_token.balance(&recipient) == 0,
            Error::MustNotHaveDebt
        );
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
        user_config: &mut UserConfiguration,
        account_data: AccountData,
        receive_stoken: bool,
    ) -> Result<(), Error> {
        let liquidation_data = account_data
            .liquidation
            .expect("pool: liquidation flag in calc_account_data");
        let mut debt_with_penalty = liquidation_data.total_debt_with_penalty_in_xlm;

        for collateral_to_receive in liquidation_data.collateral_to_receive {
            if debt_with_penalty == 0 {
                break;
            }

            let (reserve, s_token_balance, price_fixed, coll_coeff_fixed) =
                collateral_to_receive.unwrap_optimized();
            let price = FixedI128::from_inner(price_fixed);

            let s_token = STokenClient::new(env, &reserve.s_token_address);
            let underlying_asset = s_token.underlying_asset();
            let coll_coeff = FixedI128::from_inner(coll_coeff_fixed);
            let compounded_balance = coll_coeff
                .mul_int(s_token_balance)
                .ok_or(Error::LiquidateMathError)?;
            let compounded_balance_in_xlm = price
                .mul_int(compounded_balance)
                .ok_or(Error::CalcAccountDataMathError)?;

            let withdraw_amount_in_xlm = compounded_balance_in_xlm.min(debt_with_penalty);
            // no overflow as withdraw_amount_in_xlm guaranteed less or equal than debt_to_cover
            debt_with_penalty -= withdraw_amount_in_xlm;

            let (s_token_amount, underlying_amount) =
                if withdraw_amount_in_xlm != compounded_balance_in_xlm {
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

            if receive_stoken {
                let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
                let liquidator_debt = debt_token.balance(&liquidator);

                if liquidator_debt == 0 {
                    s_token.transfer_on_liquidation(&who, &liquidator, &s_token_amount);
                } else {
                    let debt_coeff = Self::get_debt_coeff(env, &reserve)?;

                    let liquidator_actual_debt = debt_coeff
                        .mul_int(liquidator_debt)
                        .ok_or(Error::LiquidateMathError)?;

                    let repayment_amount = liquidator_actual_debt.min(underlying_amount);

                    let s_token_to_burn = coll_coeff
                        .recip_mul_int(repayment_amount)
                        .ok_or(Error::LiquidateMathError)?;

                    s_token.burn(&who, &s_token_to_burn, &repayment_amount, &liquidator);

                    let (_, is_repayed) = Self::do_repay(
                        env,
                        &liquidator,
                        &underlying_asset,
                        repayment_amount,
                        Some(liquidator_debt),
                        &reserve,
                    )?;

                    if is_repayed {
                        let mut liquidator_user_config = read_user_config(env, liquidator.clone())?;
                        liquidator_user_config.set_borrowing(env, reserve.get_id(), false);
                        write_user_config(env, liquidator.clone(), &liquidator_user_config);
                    }

                    let s_token_amount = s_token_amount
                        .checked_sub(s_token_to_burn)
                        .ok_or(Error::LiquidateMathError)?;

                    if s_token_amount > 0 {
                        s_token.transfer_on_liquidation(&who, &liquidator, &s_token_amount);
                    }
                }
            } else {
                s_token.burn(&who, &s_token_amount, &underlying_amount, &liquidator);
            }

            if s_token_balance == s_token_amount {
                user_config.set_using_as_collateral(env, reserve.get_id(), false);
                event::reserve_used_as_collateral_disabled(
                    env,
                    who.clone(),
                    underlying_asset.clone(),
                );
            }

            let s_token_supply = s_token.total_supply();
            recalculate_reserve_data(env, underlying_asset, reserve, s_token_supply)?;
        }

        if debt_with_penalty != 0 {
            return Err(Error::NotEnoughCollateral);
        }

        for debt_to_cover in liquidation_data.debt_to_cover {
            let (reserve, compounded_debt, debt_amount) = debt_to_cover.unwrap_optimized();
            let s_token = STokenClient::new(env, &reserve.s_token_address);
            let s_token_supply = s_token.total_supply();
            let underlying_asset = token::Client::new(env, &s_token.underlying_asset());
            let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
            underlying_asset.transfer(&liquidator, &reserve.s_token_address, &compounded_debt);
            debt_token.burn(&who, &debt_amount);
            user_config.set_borrowing(env, reserve.get_id(), false);
            recalculate_reserve_data(env, underlying_asset.address, reserve, s_token_supply)?;
        }

        write_user_config(env, who, user_config);

        Ok(())
    }
}

/// Returns reserve data with updated accrued coeffiсients
pub fn get_actual_reserve_data(env: &Env, asset: Address) -> Result<ReserveData, Error> {
    let reserve = read_reserve(env, asset.clone())?;
    let s_token = STokenClient::new(env, &reserve.s_token_address);
    let s_token_supply = s_token.total_supply();
    recalculate_reserve_data(env, asset, reserve, s_token_supply)
}

pub fn recalculate_reserve_data(
    env: &Env,
    asset: Address,
    reserve: ReserveData,
    s_token_supply: i128,
) -> Result<ReserveData, Error> {
    let current_time = env.ledger().timestamp();
    let elapsed_time = current_time
        .checked_sub(reserve.last_update_timestamp)
        .ok_or(Error::AccruedRateMathError)?;
    if elapsed_time == 0 {
        return Ok(reserve);
    }

    let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
    let debt_token_supply = debt_token.total_supply();

    let ir_params = read_ir_params(env)?;
    let accrued_rates = calc_accrued_rates(
        s_token_supply,
        debt_token_supply,
        elapsed_time,
        ir_params,
        &reserve,
    )
    .ok_or(Error::AccruedRateMathError)?;

    let mut reserve = reserve;
    reserve.lender_accrued_rate = accrued_rates.lender_accrued_rate.into_inner();
    reserve.borrower_accrued_rate = accrued_rates.borrower_accrued_rate.into_inner();
    reserve.borrower_ir = accrued_rates.borrower_ir.into_inner();
    reserve.lender_ir = accrued_rates.lender_ir.into_inner();
    reserve.last_update_timestamp = current_time;

    write_reserve(env, asset, &reserve);
    Ok(reserve)
}
