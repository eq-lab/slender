#![deny(warnings)]
#![no_std]

use crate::price_provider::PriceProvider;
use common::{FixedI128, PERCENTAGE_FACTOR};
use debt_token_interface::DebtTokenClient;
use pool_interface::*;
use rate::{calc_accrued_rates, calc_next_accrued_rate};
use s_token_interface::STokenClient;
use soroban_sdk::{
    assert_with_error, contract, contractimpl, contracttype, panic_with_error, token, vec, Address,
    BytesN, Env, Map, Vec,
};
use user_configurator::UserConfigurator;

mod event;
mod price_provider;
mod rate;
mod storage;
mod user_configurator;

#[cfg(test)]
mod tests;

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
    pub fn default(env: &Env, liquidation: bool) -> Self {
        Self {
            discounted_collateral: 0,
            debt: 0,
            liquidation: liquidation.then_some(LiquidationData::default(env)),
            npv: 0,
        }
    }

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

#[cfg(feature = "exceeded-limit-fix")]
struct MintBurn {
    asset_balance: AssetBalance,
    mint: bool,
    who: Address,
}

#[contract]
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
        require_valid_ir_params(&env, &ir_params);

        write_admin(&env, &admin);
        write_treasury(&env, &treasury);
        write_ir_params(&env, &ir_params);

        event::initialized(&env, &admin, &treasury, ir_params);

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
        require_admin(&env)?;
        require_uninitialized_reserve(&env, &asset);

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
        write_reserve(&env, &asset, &reserve_data);

        Ok(())
    }

    /// Activates/De-activates reserve for the specified asset.
    ///
    /// # Arguments
    ///
    /// - asset - address of the asset associated with the reserve
    /// - is_active - flag indicating the reserve must be activeted or de-activated
    ///
    /// # Panics
    /// - Panics with `NoReserveExistForAsset` if no reserve exists for the specified asset.
    /// - Panics if the caller is not the admin.
    ///
    fn set_reserve_status(env: Env, asset: Address, is_active: bool) -> Result<(), Error> {
        require_admin(&env)?;

        let mut reserve = read_reserve(&env, &asset)?;

        reserve.configuration.is_active = is_active;
        write_reserve(&env, &asset, &reserve);

        if is_active {
            event::reserve_activated(&env, &asset);
        } else {
            event::reserve_deactivated(&env, &asset);
        }

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
        require_admin(&env)?;
        require_valid_ir_params(&env, &input);

        write_ir_params(&env, &input);

        Ok(())
    }

    /// Retrieves the interest rate parameters.
    ///
    /// # Returns
    ///
    /// Returns the interest rate parameters if set, or None otherwise.
    ///
    fn ir_params(env: Env) -> Option<IRParams> {
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
        require_admin(&env)?;

        let mut reserve = read_reserve(&env, &asset)?;
        reserve.configuration.borrowing_enabled = enabled;
        write_reserve(&env, &asset, &reserve);

        if enabled {
            event::borrowing_enabled(&env, &asset);
        } else {
            event::borrowing_disabled(&env, &asset);
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
        require_admin(&env)?;
        require_valid_collateral_params(&env, &params);

        let mut reserve = read_reserve(&env, &asset)?;
        reserve.update_collateral_config(params);

        write_reserve(&env, &asset, &reserve);
        event::collat_config_change(&env, &asset, params);

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
        read_reserve(&env, &asset).ok()
    }

    /// Returns collateral coefficient corrected on current time expressed as inner value of FixedI128
    ///
    /// # Arguments
    ///
    /// - asset - The address of underlying asset
    fn collat_coeff(env: Env, asset: Address) -> Result<i128, Error> {
        let reserve = read_reserve(&env, &asset)?;
        get_collat_coeff(&env, &reserve).map(|fixed| fixed.into_inner())
    }

    /// Returns debt coefficient corrected on current time expressed as inner value of FixedI128.
    /// The same as borrower accrued rate
    ///
    /// # Arguments
    ///
    /// - asset - The address of underlying asset
    fn debt_coeff(env: Env, asset: Address) -> Result<i128, Error> {
        let reserve = read_reserve(&env, &asset)?;
        get_debt_coeff(&env, &reserve).map(|fixed| fixed.into_inner())
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
        require_admin(&env)?;
        PriceProvider::new(&env, &feed);

        write_price_feed(&env, &feed, &assets);

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
    fn price_feed(env: Env, asset: Address) -> Option<Address> {
        read_price_feed(&env, &asset).ok()
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

        require_not_paused(&env);
        require_positive_amount(&env, amount);

        let reserve = get_actual_reserve_data(&env, &asset)?;
        require_active_reserve(&env, &reserve);

        let (amount, is_repayed) = do_repay(&env, &who, &asset, amount, None, &reserve)?;
        let is_first_deposit = do_deposit(&env, &who, &asset, amount, &reserve)?;

        let reserve_id = reserve.get_id();
        UserConfigurator::new(&env, &who, true)
            .repay(reserve_id, is_repayed)?
            .deposit(reserve_id, &asset, is_first_deposit)?
            .write();

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
        require_not_paused(&env);

        let reserve = read_reserve(&env, &asset)?;
        require_active_reserve(&env, &reserve);
        require_zero_debt(&env, &to, &reserve.debt_token_address);

        reserve.s_token_address.require_auth();

        let reserve = recalculate_reserve_data(&env, &asset, &reserve, s_token_supply)?;

        let balance_from_after = balance_from_before
            .checked_sub(amount)
            .ok_or(Error::InvalidAmount)?;

        let mut from_configurator = UserConfigurator::new(&env, &from, false);
        let from_config = from_configurator.user_config()?;

        if from_config.is_borrowing_any()
            && from_config.is_using_as_collateral(&env, reserve.get_id())
        {
            let reserves = read_reserves(&env);
            let from_account_data = calc_account_data(
                &env,
                &from,
                Some(&AssetBalance::new(
                    reserve.s_token_address.clone(),
                    balance_from_after,
                )),
                from_config,
                &reserves,
                false,
            )?;

            require_good_position(&env, &from_account_data);
        }

        if from != to {
            let reserve_id = reserve.get_id();
            let is_to_deposit = balance_to_before == 0 && amount != 0;

            from_configurator
                .withdraw(reserve_id, &asset, balance_from_after == 0)?
                .write();

            UserConfigurator::new(&env, &to, true)
                .deposit(reserve_id, &asset, is_to_deposit)?
                .write();
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

        require_not_paused(&env);
        require_positive_amount(&env, amount);

        let reserve = get_actual_reserve_data(&env, &asset)?;
        require_active_reserve(&env, &reserve);

        let s_token = STokenClient::new(&env, &reserve.s_token_address);
        let s_token_balance = s_token.balance(&who);

        let collat_coeff = get_collat_coeff(&env, &reserve)?;
        let underlying_balance = collat_coeff
            .mul_int(s_token_balance)
            .ok_or(Error::MathOverflowError)?;

        let (underlying_to_withdraw, s_token_to_burn) = if amount == i128::MAX {
            (underlying_balance, s_token_balance)
        } else {
            let s_token_to_burn = collat_coeff
                .recip_mul_int(amount)
                .ok_or(Error::MathOverflowError)?;
            (amount, s_token_to_burn)
        };

        assert_with_error!(
            env,
            underlying_to_withdraw <= underlying_balance,
            Error::NotEnoughAvailableUserBalance
        );

        let mut user_configurator = UserConfigurator::new(&env, &who, false);
        let s_token_balance_after = s_token_balance
            .checked_sub(s_token_to_burn)
            .ok_or(Error::InvalidAmount)?;

        require_good_position_when_withdrawing(
            &env,
            &who,
            &reserve,
            user_configurator.user_config()?,
            &AssetBalance::new(s_token.address.clone(), s_token_balance_after),
        )?;

        let amount_to_sub = underlying_to_withdraw
            .checked_neg()
            .ok_or(Error::MathOverflowError)?;

        s_token.burn(&who, &s_token_to_burn, &underlying_to_withdraw, &to);
        add_stoken_underlying_balance(&env, &s_token.address, amount_to_sub)?;

        let is_full_withdraw = underlying_to_withdraw == underlying_balance;
        user_configurator
            .withdraw(reserve.get_id(), &asset, is_full_withdraw)?
            .write();

        event::withdraw(&env, &who, &asset, &to, underlying_to_withdraw);

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

        require_not_paused(&env);
        require_positive_amount(&env, amount);

        let reserve = get_actual_reserve_data(&env, &asset)?;
        require_active_reserve(&env, &reserve);
        require_borrowing_enabled(&env, &reserve);

        let s_token = STokenClient::new(&env, &reserve.s_token_address);
        let debt_token = DebtTokenClient::new(&env, &reserve.debt_token_address);
        let mut user_configurator = UserConfigurator::new(&env, &who, false);

        require_good_position_when_borrowing(
            &env,
            &who,
            &asset,
            amount,
            &s_token,
            &debt_token,
            &reserve,
            user_configurator.user_config()?,
        )?;

        let debt_coeff = get_debt_coeff(&env, &reserve)?;
        let amount_of_debt_token = debt_coeff
            .recip_mul_int(amount)
            .ok_or(Error::MathOverflowError)?;

        let is_first_borrowing = debt_token.balance(&who) == 0;
        let amount_to_sub = amount.checked_neg().ok_or(Error::MathOverflowError)?;

        debt_token.mint(&who, &amount_of_debt_token);
        s_token.transfer_underlying_to(&who, &amount);
        add_stoken_underlying_balance(&env, &s_token.address, amount_to_sub)?;

        user_configurator
            .borrow(reserve.get_id(), is_first_borrowing)?
            .write();

        event::borrow(&env, &who, &asset, amount);

        Ok(())
    }

    fn set_pause(env: Env, value: bool) -> Result<(), Error> {
        require_admin(&env)?;
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
    fn account_position(env: Env, who: Address) -> Result<AccountPosition, Error> {
        let account_data = calc_account_data(
            &env,
            &who,
            None,
            &read_user_config(&env, &who)?,
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

        require_not_paused(&env);

        let reserves = read_reserves(&env);
        let mut user_configurator = UserConfigurator::new(&env, &who, false);
        let account_data = calc_account_data(
            &env,
            &who,
            None,
            user_configurator.user_config()?,
            &reserves,
            true,
        )?;

        assert_with_error!(&env, !account_data.is_good_position(), Error::GoodPosition);

        let liquidation = account_data
            .liquidation
            .expect("pool: liquidation flag in calc_account_data");

        do_liquidate(
            &env,
            &liquidator,
            &who,
            &mut user_configurator,
            &liquidation,
            receive_stoken,
        )?;

        event::liquidation(
            &env,
            &who,
            account_data.debt,
            liquidation.total_debt_with_penalty_in_xlm,
        );

        Ok(())
    }

    /// Enables or disables asset for using as collateral.
    /// User should not have the debt in asset.
    /// If user has debt position it will be checked if position stays good after disabling collateral.
    ///
    /// # Arguments
    /// - who The address for collateral enabling/disabling
    /// - asset The address of underlying asset
    /// - use_as_collateral Enable/disable flag
    ///
    /// # Errors
    /// - UserConfigNotExists
    /// - NoReserveExistForAsset
    /// - MustNotHaveDebt
    /// - Bad position
    ///
    fn set_as_collateral(
        env: Env,
        who: Address,
        asset: Address,
        use_as_collateral: bool,
    ) -> Result<(), Error> {
        who.require_auth();

        let mut user_configurator = UserConfigurator::new(&env, &who, false);
        let user_config = user_configurator.user_config()?;
        let reserve_id = read_reserve(&env, &asset)?.get_id();

        assert_with_error!(
            &env,
            !user_config.is_borrowing(&env, reserve_id),
            Error::MustNotHaveDebt
        );

        if !use_as_collateral
            && user_config.is_borrowing_any()
            && user_config.is_using_as_collateral(&env, reserve_id)
        {
            user_configurator.withdraw(reserve_id, &asset, true)?;

            let account_data = calc_account_data(
                &env,
                &who,
                None,
                user_configurator.user_config()?,
                &read_reserves(&env),
                false,
            )?;

            require_good_position(&env, &account_data);

            user_configurator.write();

            return Ok(());
        }

        user_configurator
            .deposit(reserve_id, &asset, use_as_collateral)?
            .withdraw(reserve_id, &asset, !use_as_collateral)?
            .write();

        Ok(())
    }

    /// Retrieves the user configuration.
    ///
    /// # Arguments
    /// - who The address for which the configuration is getting
    ///
    /// # Errors
    /// - UserConfigNotExists
    ///
    /// # Returns
    ///
    /// Returns the user configuration:
    /// bitmask where even/odd bits correspond to reserve indexes and indicate whether collateral/borrow is allowed for this reserve.
    ///
    fn user_configuration(env: Env, who: Address) -> Result<UserConfiguration, Error> {
        read_user_config(&env, &who)
    }

    fn stoken_underlying_balance(env: Env, stoken_address: Address) -> i128 {
        read_stoken_underlying_balance(&env, &stoken_address)
    }
}

fn require_admin(env: &Env) -> Result<(), Error> {
    let admin: Address = read_admin(env)?;
    admin.require_auth();
    Ok(())
}

fn require_valid_ir_params(env: &Env, params: &IRParams) {
    require_lte_percentage_factor(env, params.initial_rate);
    require_gt_percentage_factor(env, params.max_rate);
    require_lt_percentage_factor(env, params.scaling_coeff);
}

fn require_valid_collateral_params(env: &Env, params: &CollateralParamsInput) {
    require_lte_percentage_factor(env, params.discount);
    require_lte_percentage_factor(env, params.util_cap);
    require_gt_percentage_factor(env, params.liq_bonus);
    require_positive(env, params.liq_cap);
}

fn require_uninitialized_reserve(env: &Env, asset: &Address) {
    assert_with_error!(
        env,
        !has_reserve(env, asset),
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

fn require_positive_amount(env: &Env, amount: i128) {
    assert_with_error!(env, amount > 0, Error::InvalidAmount);
}

fn require_active_reserve(env: &Env, reserve: &ReserveData) {
    assert_with_error!(env, reserve.configuration.is_active, Error::NoActiveReserve);
}

fn require_borrowing_enabled(env: &Env, reserve: &ReserveData) {
    assert_with_error!(
        env,
        reserve.configuration.borrowing_enabled,
        Error::BorrowingNotEnabled
    );
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

        FixedI128::from_inner(reserve.lender_ar)
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

fn require_util_cap_not_exceeded(
    env: &Env,
    s_token: &STokenClient,
    debt_token: &DebtTokenClient,
    util_cap: u32,
    amount: i128,
) -> Result<(), Error> {
    let total_debt_after = debt_token
        .total_supply()
        .checked_add(amount)
        .ok_or(Error::ValidateBorrowMathError)?;
    let total_collat = s_token.total_supply();
    let utilization = FixedI128::from_rational(total_debt_after, total_collat)
        .ok_or(Error::ValidateBorrowMathError)?;
    let util_cap = FixedI128::from_percentage(util_cap).ok_or(Error::ValidateBorrowMathError)?;

    assert_with_error!(env, utilization <= util_cap, Error::UtilizationCapExceeded);

    Ok(())
}

fn require_good_position(env: &Env, account_data: &AccountData) {
    assert_with_error!(env, account_data.is_good_position(), Error::BadPosition);
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

    let balance = read_stoken_underlying_balance(env, &reserve.s_token_address);

    require_liq_cap_not_exceeded(env, reserve, balance, amount)?;

    let s_token = STokenClient::new(env, &reserve.s_token_address);
    let is_first_deposit = s_token.balance(who) == 0;

    let collat_coeff = get_collat_coeff(env, reserve)?;
    let amount_to_mint = collat_coeff
        .recip_mul_int(amount)
        .ok_or(Error::MathOverflowError)?;

    let underlying_asset = token::Client::new(env, asset);

    underlying_asset.transfer(who, &s_token.address, &amount);
    add_stoken_underlying_balance(env, &s_token.address, amount)?;
    s_token.mint(who, &amount_to_mint);

    event::deposit(env, who, asset, amount);

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

    let debt_coeff = get_debt_coeff(env, reserve)?;
    let collat_coeff = get_collat_coeff(env, reserve)?;

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
    add_stoken_underlying_balance(env, &reserve.s_token_address, lender_part)?;
    underlying_asset.transfer(who, &treasury_address, &treasury_part);
    debt_token.burn(who, &borrower_debt_to_burn);

    event::repay(env, who, asset, borrower_payback_amount);

    let remaning_amount = if amount != i128::MAX && amount > borrower_actual_debt {
        amount
            .checked_sub(borrower_actual_debt)
            .ok_or(Error::MathOverflowError)?
    } else {
        0
    };

    Ok((remaning_amount, is_repayed))
}

fn require_good_position_when_withdrawing(
    env: &Env,
    who: &Address,
    reserve: &ReserveData,
    user_config: &UserConfiguration,
    s_token_after: &AssetBalance,
) -> Result<(), Error> {
    if !user_config.is_borrowing_any() {
        return Ok(());
    }

    if !user_config.is_using_as_collateral(env, reserve.get_id()) {
        return Ok(());
    }

    let account_data = calc_account_data(
        env,
        who,
        Some(s_token_after),
        user_config,
        &read_reserves(env),
        false,
    )?;

    require_good_position(env, &account_data);

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn require_good_position_when_borrowing(
    env: &Env,
    who: &Address,
    asset: &Address,
    amount: i128,
    s_token: &STokenClient,
    debt_token: &DebtTokenClient,
    reserve: &ReserveData,
    user_config: &UserConfiguration,
) -> Result<(), Error> {
    // `is_using_as_collateral` is skipped to avoid case when user:
    // makes deposit => disables `is_using_as_collateral` => borrows the asset
    let stoken_balance = s_token.balance(who);
    assert_with_error!(
        env,
        s_token.balance(who) == 0,
        Error::MustNotBeInCollateralAsset
    );

    let util_cap = reserve.configuration.util_cap;
    require_util_cap_not_exceeded(env, s_token, debt_token, util_cap, amount)?;

    let asset_price = get_asset_price(env, asset)?;
    let amount_in_xlm = asset_price
        .mul_int(amount)
        .ok_or(Error::ValidateBorrowMathError)?;

    require_positive_amount(env, amount_in_xlm);

    let account_data = calc_account_data(
        env,
        who,
        Some(&AssetBalance::new(s_token.address.clone(), stoken_balance)),
        user_config,
        &read_reserves(env),
        false,
    )?;

    assert_with_error!(
        env,
        account_data.npv >= amount_in_xlm,
        Error::CollateralNotCoverNewBorrow
    );

    Ok(())
}

fn calc_account_data(
    env: &Env,
    who: &Address,
    mb_who_balance: Option<&AssetBalance>,
    user_config: &UserConfiguration,
    reserves: &Vec<Address>,
    liquidation: bool,
) -> Result<AccountData, Error> {
    if user_config.is_empty() {
        return Ok(AccountData::default(env, liquidation));
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

        let curr_reserve_asset = reserves.get_unchecked(i.into());
        let curr_reserve = read_reserve(env, &curr_reserve_asset)?;

        assert_with_error!(
            env,
            curr_reserve.configuration.is_active || !liquidation,
            Error::NoActiveReserve
        );

        let reserve_price = get_asset_price(env, &curr_reserve_asset)?;

        if user_config.is_using_as_collateral(env, i) {
            let collat_coeff = get_collat_coeff(env, &curr_reserve)?;

            let who_balance: i128 = match &mb_who_balance {
                Some(AssetBalance { asset, balance }) if *asset == curr_reserve.s_token_address => {
                    *balance
                }
                _ => STokenClient::new(env, &curr_reserve.s_token_address).balance(who),
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
                    .unwrap_or(Vec::new(env));
                collateral_to_receive.push_back((
                    curr_reserve,
                    who_balance,
                    reserve_price.into_inner(),
                    collat_coeff.into_inner(),
                ));
                sorted_collateral_to_receive.set(curr_discount, collateral_to_receive);
            }
        } else if user_config.is_borrowing(env, i) {
            let debt_coeff = get_debt_coeff(env, &curr_reserve)?;

            let debt_token = token::Client::new(env, &curr_reserve.debt_token_address);
            let debt_token_balance = debt_token.balance(who);
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
                let liq_bonus = FixedI128::from_percentage(curr_reserve.configuration.liq_bonus)
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
            for c in v {
                collateral_to_receive.push_back(c);
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
fn get_asset_price(env: &Env, asset: &Address) -> Result<FixedI128, Error> {
    let price_feed = read_price_feed(env, asset)?;
    let provider = PriceProvider::new(env, &price_feed);

    provider.get_price(asset).map(|price_data| {
        FixedI128::from_rational(price_data.price, 10i128.pow(price_data.decimals))
            .ok_or(Error::AssetPriceMathError)
    })?
}

/// Returns lender accrued rate corrected for the current time
fn get_actual_lender_accrued_rate(env: &Env, reserve: &ReserveData) -> Result<FixedI128, Error> {
    let current_time = env.ledger().timestamp();
    let elapsed_time = current_time
        .checked_sub(reserve.last_update_timestamp)
        .ok_or(Error::CollateralCoeffMathError)?;
    let prev_ar = FixedI128::from_inner(reserve.lender_ar);

    if elapsed_time == 0 {
        Ok(prev_ar)
    } else {
        let lender_ir = FixedI128::from_inner(reserve.lender_ir);
        calc_next_accrued_rate(prev_ar, lender_ir, elapsed_time)
            .ok_or(Error::CollateralCoeffMathError)
    }
}

/// Returns collateral coefficient
/// collateral_coeff = [underlying_balance + lender_ar * total_debt_token]/total_stoken
fn get_collat_coeff(env: &Env, reserve: &ReserveData) -> Result<FixedI128, Error> {
    let s_token = STokenClient::new(env, &reserve.s_token_address);
    let s_token_supply = s_token.total_supply();

    if s_token_supply == 0 {
        return Ok(FixedI128::ONE);
    }

    let collat_ar = get_actual_lender_accrued_rate(env, reserve)?;

    let balance = read_stoken_underlying_balance(env, &reserve.s_token_address);
    let debt_token_supply = DebtTokenClient::new(env, &reserve.debt_token_address).total_supply();

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
fn get_actual_borrower_accrued_rate(env: &Env, reserve: &ReserveData) -> Result<FixedI128, Error> {
    let current_time = env.ledger().timestamp();
    let elapsed_time = current_time
        .checked_sub(reserve.last_update_timestamp)
        .ok_or(Error::DebtCoeffMathError)?;
    let prev_ar = FixedI128::from_inner(reserve.borrower_ar);

    if elapsed_time == 0 {
        Ok(prev_ar)
    } else {
        let debt_ir = FixedI128::from_inner(reserve.borrower_ir);
        calc_next_accrued_rate(prev_ar, debt_ir, elapsed_time).ok_or(Error::DebtCoeffMathError)
    }
}

/// The same as borrower accrued rate
fn get_debt_coeff(env: &Env, reserve: &ReserveData) -> Result<FixedI128, Error> {
    get_actual_borrower_accrued_rate(env, reserve)
}

fn require_not_paused(env: &Env) {
    assert_with_error!(env, !paused(env), Error::Paused);
}

fn require_zero_debt(env: &Env, recipient: &Address, debt_token_address: &Address) {
    let debt_token = DebtTokenClient::new(env, debt_token_address);
    assert_with_error!(
        env,
        debt_token.balance(recipient) == 0,
        Error::MustNotHaveDebt
    );
}

fn do_liquidate(
    env: &Env,
    liquidator: &Address,
    who: &Address,
    user_configurator: &mut UserConfigurator,
    liquidation_data: &LiquidationData,
    receive_stoken: bool,
) -> Result<(), Error> {
    let mut debt_with_penalty = liquidation_data.total_debt_with_penalty_in_xlm;

    for (reserve, s_token_balance, price_fixed, coll_coeff_fixed) in
        liquidation_data.collateral_to_receive.iter()
    {
        if debt_with_penalty == 0 {
            break;
        }

        let price = FixedI128::from_inner(price_fixed);

        let s_token = STokenClient::new(env, &reserve.s_token_address);
        let asset = s_token.underlying_asset();
        let coll_coeff = FixedI128::from_inner(coll_coeff_fixed);
        let compounded_balance = coll_coeff
            .mul_int(s_token_balance)
            .ok_or(Error::LiquidateMathError)?;
        let compounded_balance_in_xlm = price
            .mul_int(compounded_balance)
            .ok_or(Error::CalcAccountDataMathError)?;

        let withdraw_amount_in_xlm = compounded_balance_in_xlm.min(debt_with_penalty);
        // no overflow as withdraw_amount_in_xlm guaranteed less or equal to debt_to_cover
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
            let liquidator_debt = debt_token.balance(liquidator);
            let liquidator_collat_before = s_token.balance(liquidator);

            let mut liquidator_collat_amount = s_token_amount;
            let mut is_debt_repayed = false;

            if liquidator_debt > 0 {
                let debt_coeff = get_debt_coeff(env, &reserve)?;

                let liquidator_actual_debt = debt_coeff
                    .mul_int(liquidator_debt)
                    .ok_or(Error::LiquidateMathError)?;

                let repayment_amount = liquidator_actual_debt.min(underlying_amount);

                let s_token_to_burn = coll_coeff
                    .recip_mul_int(repayment_amount)
                    .ok_or(Error::LiquidateMathError)?;

                let amount_to_sub = repayment_amount
                    .checked_neg()
                    .ok_or(Error::MathOverflowError)?;

                s_token.burn(who, &s_token_to_burn, &repayment_amount, liquidator);
                add_stoken_underlying_balance(env, &s_token.address, amount_to_sub)?;

                let (_, is_repayed) = do_repay(
                    env,
                    liquidator,
                    &asset,
                    repayment_amount,
                    Some(liquidator_debt),
                    &reserve,
                )?;

                is_debt_repayed = is_repayed;

                liquidator_collat_amount = s_token_amount
                    .checked_sub(s_token_to_burn)
                    .ok_or(Error::LiquidateMathError)?;
            }

            if liquidator_collat_amount > 0 {
                s_token.transfer_on_liquidation(who, liquidator, &liquidator_collat_amount);
            }

            let use_as_collat = liquidator_collat_before == 0 && liquidator_collat_amount > 0;
            let reserve_id = reserve.get_id();

            UserConfigurator::new(env, liquidator, true)
                .deposit(reserve_id, &asset, use_as_collat)?
                .repay(reserve_id, is_debt_repayed)?
                .write();
        } else {
            let amount_to_sub = underlying_amount
                .checked_neg()
                .ok_or(Error::MathOverflowError)?;

            s_token.burn(who, &s_token_amount, &underlying_amount, liquidator);
            add_stoken_underlying_balance(env, &s_token.address, amount_to_sub)?;
        }

        let is_withdraw = s_token_balance == s_token_amount;
        user_configurator.withdraw(reserve.get_id(), &asset, is_withdraw)?;

        let s_token_supply = s_token.total_supply();
        recalculate_reserve_data(env, &asset, &reserve, s_token_supply)?;
    }

    assert_with_error!(env, debt_with_penalty == 0, Error::NotEnoughCollateral);

    for (reserve, compounded_debt, debt_amount) in liquidation_data.debt_to_cover.iter() {
        let s_token = STokenClient::new(env, &reserve.s_token_address);
        let s_token_supply = s_token.total_supply();
        let underlying_asset = token::Client::new(env, &s_token.underlying_asset());
        let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);

        underlying_asset.transfer(liquidator, &reserve.s_token_address, &compounded_debt);
        add_stoken_underlying_balance(env, &s_token.address, compounded_debt)?;
        debt_token.burn(who, &debt_amount);
        user_configurator.repay(reserve.get_id(), true)?;

        recalculate_reserve_data(env, &underlying_asset.address, &reserve, s_token_supply)?;
    }

    user_configurator.write();

    Ok(())
}

/// Returns reserve data with updated accrued coeffiсients
fn get_actual_reserve_data(env: &Env, asset: &Address) -> Result<ReserveData, Error> {
    let reserve = read_reserve(env, asset)?;
    let s_token = STokenClient::new(env, &reserve.s_token_address);
    let s_token_supply = s_token.total_supply();

    recalculate_reserve_data(env, asset, &reserve, s_token_supply)
}

fn recalculate_reserve_data(
    env: &Env,
    asset: &Address,
    reserve: &ReserveData,
    s_token_supply: i128,
) -> Result<ReserveData, Error> {
    let current_time = env.ledger().timestamp();
    let elapsed_time = current_time
        .checked_sub(reserve.last_update_timestamp)
        .ok_or(Error::AccruedRateMathError)?;

    if elapsed_time == 0 || s_token_supply == 0 {
        return Ok(reserve.clone());
    }

    let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
    let debt_token_supply = debt_token.total_supply();

    let ir_params = read_ir_params(env)?;
    let accrued_rates = calc_accrued_rates(
        s_token_supply,
        debt_token_supply,
        elapsed_time,
        ir_params,
        reserve,
    )
    .ok_or(Error::AccruedRateMathError)?;

    let mut reserve = reserve.clone();
    reserve.lender_ar = accrued_rates.lender_ar.into_inner();
    reserve.borrower_ar = accrued_rates.borrower_ar.into_inner();
    reserve.borrower_ir = accrued_rates.borrower_ir.into_inner();
    reserve.lender_ir = accrued_rates.lender_ir.into_inner();
    reserve.last_update_timestamp = current_time;

    write_reserve(env, asset, &reserve);

    Ok(reserve)
}
