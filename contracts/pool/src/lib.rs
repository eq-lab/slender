#![deny(warnings)]
#![no_std]

#[cfg(not(feature = "exceeded-limit-fix"))]
use methods::{
    account_position::account_position, borrow::borrow, collat_coeff::collat_coeff,
    deposit::deposit, finalize_transfer::finalize_transfer, flash_loan::flash_loan,
    liquidate::liquidate, repay::repay, set_as_collateral::set_as_collateral, withdraw::withdraw,
};
use methods::{
    configure_as_collateral::configure_as_collateral, debt_coeff::debt_coeff,
    enable_borrowing_on_reserve::enable_borrowing_on_reserve, init_reserve::init_reserve,
    initialize::initialize, set_decimals::set_decimals, set_flash_loan_fee::set_flash_loan_fee,
    set_ir_params::set_ir_params, set_pause::set_pause, set_price_feed::set_price_feed,
    set_reserve_status::set_reserve_status, upgrade::upgrade,
    upgrade_debt_token::upgrade_debt_token, upgrade_s_token::upgrade_s_token,
};
#[cfg(feature = "exceeded-limit-fix")]
use methods::{
    fix_limit::account_position::account_position, fix_limit::borrow::borrow,
    fix_limit::collat_coeff::collat_coeff, fix_limit::deposit::deposit,
    fix_limit::finalize_transfer::finalize_transfer, fix_limit::flash_loan::flash_loan,
    fix_limit::liquidate::liquidate, fix_limit::repay::repay,
    fix_limit::set_as_collateral::set_as_collateral, fix_limit::withdraw::withdraw,
};
#[cfg(feature = "exceeded-limit-fix")]
use pool_interface::types::mint_burn::MintBurn;
use pool_interface::types::{
    account_position::AccountPosition, collateral_params_input::CollateralParamsInput,
    error::Error, flash_loan_asset::FlashLoanAsset, init_reserve_input::InitReserveInput,
    ir_params::IRParams, reserve_data::ReserveData, user_config::UserConfiguration,
};
use pool_interface::LendingPoolTrait;
use soroban_sdk::{contract, contractimpl, Address, Bytes, BytesN, Env, Vec};

use crate::storage::*;

mod event;
mod methods;
mod storage;
#[cfg(test)]
mod tests;
mod types;

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
    /// - flash_loan_fee - Кepresents the fee paid by the flash loan borrowers.
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
        flash_loan_fee: u32,
        ir_params: IRParams,
    ) -> Result<(), Error> {
        initialize(&env, &admin, &treasury, flash_loan_fee, &ir_params)
    }

    /// Upgrades the deployed contract wasm preserving the contract id.
    ///
    /// # Arguments
    ///
    /// - new_wasm_hash - The new version of the WASM hash.
    ///
    /// # Panics
    ///
    /// - Panics with `Uninitialized` if the admin key is not exist in storage.
    /// - Panics if the caller is not the admin.
    ///
    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
        upgrade(&env, &new_wasm_hash)
    }

    /// Upgrades the deployed s_token contract wasm preserving the contract id.
    ///
    /// # Arguments
    ///
    /// - new_wasm_hash - The new version of the WASM hash.
    /// - asset - The address of the asset associated with the reserve.
    ///
    /// # Panics
    ///
    /// - Panics with `Uninitialized` if the admin key is not exist in storage.
    /// - Panics with `NoReserveExistForAsset` if no reserve exists for the specified asset.
    /// - Panics if the caller is not the admin.
    ///
    fn upgrade_s_token(env: Env, asset: Address, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
        upgrade_s_token(&env, &asset, &new_wasm_hash)
    }

    /// Upgrades the deployed debt_token contract wasm preserving the contract id.
    ///
    /// # Arguments
    ///
    /// - new_wasm_hash - The new version of the WASM hash.
    /// - asset - The address of the asset associated with the reserve.
    ///
    /// # Panics
    ///
    /// - Panics with `Uninitialized` if the admin key is not exist in storage.
    /// - Panics with `NoReserveExistForAsset` if no reserve exists for the specified asset.
    /// - Panics if the caller is not the admin.
    ///
    fn upgrade_debt_token(
        env: Env,
        asset: Address,
        new_wasm_hash: BytesN<32>,
    ) -> Result<(), Error> {
        upgrade_debt_token(&env, &asset, &new_wasm_hash)
    }

    /// Returns the current version of the contract.
    fn version() -> u32 {
        1
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
        init_reserve(&env, &asset, &input)
    }

    /// Set decimals that used by reserve for a given asset
    ///
    /// # Arguments
    /// - asset - The address of the asset associated with the reserve.
    /// - decimals - New decimals value
    ///
    /// - Panics with `Uninitialized` if the admin key is not exist in storage.
    /// - Panics with `NoReserveExistForAsset` if no reserve exists for the specified asset.
    /// - Panics if the caller is not the admin.
    ///
    fn set_decimals(env: Env, asset: Address, decimals: u32) -> Result<(), Error> {
        set_decimals(&env, &asset, decimals)
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
        set_reserve_status(&env, &asset, is_active)
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
        set_ir_params(&env, &input)
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
        enable_borrowing_on_reserve(&env, &asset, enabled)
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
        configure_as_collateral(&env, &asset, &params)
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
        collat_coeff(&env, &asset)
    }

    /// Returns debt coefficient corrected on current time expressed as inner value of FixedI128.
    /// The same as borrower accrued rate
    ///
    /// # Arguments
    ///
    /// - asset - The address of underlying asset
    fn debt_coeff(env: Env, asset: Address) -> Result<i128, Error> {
        debt_coeff(&env, &asset)
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
        set_price_feed(&env, &feed, &assets)
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

    /// Deposits a specified amount of an asset into the reserve associated with the asset.
    /// Depositor receives s-tokens according to the current index value.
    ///
    ///
    /// # Arguments
    ///
    /// - who - The address of the user making the deposit.
    /// - asset - The address of the asset to be deposited for lend.
    /// - amount - The amount to be deposited.
    ///
    /// # Errors
    ///
    /// Returns `NoReserveExistForAsset` if no reserve exists for the specified asset.
    /// Returns `MathOverflowError' if an overflow occurs when calculating the amount of tokens.
    /// Returns `MustNotHaveDebt` if user already has debt.
    ///
    /// # Panics
    ///
    /// If the caller is not authorized.
    /// If the deposit amount is invalid or does not meet the reserve requirements.
    /// If the reserve data cannot be retrieved from storage.
    ///
    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn deposit(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error> {
        deposit(&env, &who, &asset, amount)
    }

    #[cfg(feature = "exceeded-limit-fix")]
    fn deposit(
        env: Env,
        who: Address,
        asset: Address,
        amount: i128,
    ) -> Result<Vec<MintBurn>, Error> {
        deposit(&env, &who, &asset, amount)
    }

    /// Repays a borrowed amount on a specific reserve, burning the equivalent debt tokens owned.
    ///
    ///
    /// # Arguments
    ///
    /// - who - The address of the user making the repayment.
    /// - asset - The address of the asset to be repayed.
    /// - amount - The amount to be repayed. Use i128::MAX to repay the maximum available amount.
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
    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn repay(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error> {
        repay(&env, &who, &asset, amount)
    }

    /// Repays a borrowed amount on a specific reserve, burning the equivalent debt tokens owned.
    ///
    ///
    /// # Arguments
    ///
    /// - who - The address of the user making the repayment.
    /// - asset - The address of the asset to be repayed.
    /// - amount - The amount to be repayed. Use i128::MAX to repay the maximum available amount.
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
    #[cfg(feature = "exceeded-limit-fix")]
    fn repay(env: Env, who: Address, asset: Address, amount: i128) -> Result<Vec<MintBurn>, Error> {
        repay(&env, &who, &asset, amount)
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
        finalize_transfer(
            &env,
            &asset,
            &from,
            &to,
            amount,
            balance_from_before,
            balance_to_before,
            s_token_supply,
        )
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
    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn withdraw(
        env: Env,
        who: Address,
        asset: Address,
        amount: i128,
        to: Address,
    ) -> Result<(), Error> {
        withdraw(&env, &who, &asset, amount, &to)
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
    #[cfg(feature = "exceeded-limit-fix")]
    fn withdraw(
        env: Env,
        who: Address,
        asset: Address,
        amount: i128,
        to: Address,
    ) -> Result<Vec<MintBurn>, Error> {
        withdraw(&env, &who, &asset, amount, &to)
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
    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn borrow(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error> {
        borrow(&env, &who, &asset, amount)
    }

    #[cfg(feature = "exceeded-limit-fix")]
    fn borrow(
        env: Env,
        who: Address,
        asset: Address,
        amount: i128,
    ) -> Result<Vec<MintBurn>, Error> {
        borrow(&env, &who, &asset, amount)
    }

    fn set_pause(env: Env, value: bool) -> Result<(), Error> {
        set_pause(&env, value)
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
        account_position(&env, &who)
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
    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn liquidate(
        env: Env,
        liquidator: Address,
        who: Address,
        receive_stoken: bool,
    ) -> Result<(), Error> {
        liquidate(&env, &liquidator, &who, receive_stoken)
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
    #[cfg(feature = "exceeded-limit-fix")]
    fn liquidate(
        env: Env,
        liquidator: Address,
        who: Address,
        receive_stoken: bool,
    ) -> Result<Vec<MintBurn>, Error> {
        liquidate(&env, &liquidator, &who, receive_stoken)
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
        set_as_collateral(&env, &who, &asset, use_as_collateral)
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

    #[cfg(feature = "exceeded-limit-fix")]
    fn set_price(env: Env, asset: Address, price: i128) {
        write_price(&env, &asset, price);
    }

    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn set_price(_env: Env, _asset: Address, _price: i128) {
        unimplemented!()
    }

    /// Sets the flash loan fee.
    ///
    /// # Arguments
    ///
    /// - fee - The flash loan fee in base points.
    ///
    /// # Panics
    ///
    /// - Panics with `Uninitialized` if the admin key is not exist in storage.
    /// - Panics if the caller is not the admin.
    ///
    fn set_flash_loan_fee(env: Env, fee: u32) -> Result<(), Error> {
        set_flash_loan_fee(&env, fee)
    }

    /// Retrieves the flash loan fee.
    ///
    /// # Returns
    ///
    /// Returns the flash loan fee in base points:
    ///
    fn flash_loan_fee(env: Env) -> u32 {
        read_flash_loan_fee(&env)
    }

    /// Allows the end-users to borrow the assets within one transaction
    /// ensuring the the amount taken + fee is returned.
    ///
    /// # Arguments
    /// - receiver - The contract address that implements the FlashLoanReceiverTrait
    /// and receives the requested assets.
    /// - assets - The assets being flash borrowed. If the `borrow` flag is set to true,
    /// opens debt for the flash-borrowed amount to the `who` address.
    /// - params - An extra information for the receiver.
    ///
    /// # Panics
    ///
    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn flash_loan(
        env: Env,
        who: Address,
        receiver: Address,
        loan_assets: Vec<FlashLoanAsset>,
        params: Bytes,
    ) -> Result<(), Error> {
        flash_loan(&env, &who, &receiver, &loan_assets, &params)
    }

    /// Allows the end-users to borrow the assets within one transaction
    /// ensuring the the amount taken + fee is returned.
    ///
    /// # Arguments
    /// - receiver - The contract address that implements the FlashLoanReceiverTrait
    /// and receives the requested assets.
    /// - assets - The assets being flash borrowed. If the `borrow` flag is set to true,
    /// opens debt for the flash-borrowed amount to the `who` address.
    /// - params - An extra information for the receiver.
    ///
    /// # Panics
    ///
    #[cfg(feature = "exceeded-limit-fix")]
    fn flash_loan(
        env: Env,
        who: Address,
        receiver: Address,
        loan_assets: Vec<FlashLoanAsset>,
        params: Bytes,
    ) -> Result<Vec<MintBurn>, Error> {
        flash_loan(&env, &who, &receiver, &loan_assets, &params)
    }

    #[cfg(feature = "exceeded-limit-fix")]
    fn get_price(env: Env, asset: Address) -> i128 {
        read_price(&env, &asset)
    }

    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn get_price(_env: Env, _asset: Address) -> i128 {
        unimplemented!()
    }
}
