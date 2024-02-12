#![deny(warnings)]
#![no_std]

use methods::{
    account_position::account_position, borrow::borrow, collat_coeff::collat_coeff,
    configure_as_collateral::configure_as_collateral, debt_coeff::debt_coeff, deposit::deposit,
    enable_borrowing_on_reserve::enable_borrowing_on_reserve, finalize_transfer::finalize_transfer,
    flash_loan::flash_loan, init_reserve::init_reserve, initialize::initialize,
    liquidate::liquidate, repay::repay, set_as_collateral::set_as_collateral,
    set_base_asset::set_base_asset, set_flash_loan_fee::set_flash_loan_fee,
    set_initial_health::set_initial_health, set_ir_params::set_ir_params, set_pause::set_pause,
    set_price_feeds::set_price_feeds, set_reserve_status::set_reserve_status,
    set_reserve_timestamp_window::set_reserve_timestamp_window,
    twap_median_price::twap_median_price, upgrade::upgrade, upgrade_debt_token::upgrade_debt_token,
    upgrade_s_token::upgrade_s_token, withdraw::withdraw,
};
use pool_interface::types::reserve_type::ReserveType;
use pool_interface::types::{
    account_position::AccountPosition, base_asset_config::BaseAssetConfig,
    collateral_params_input::CollateralParamsInput, error::Error, flash_loan_asset::FlashLoanAsset,
    ir_params::IRParams, price_feed_config::PriceFeedConfig,
    price_feed_config_input::PriceFeedConfigInput, reserve_data::ReserveData,
    user_config::UserConfiguration,
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
    fn initialize(
        env: Env,
        admin: Address,
        treasury: Address,
        flash_loan_fee: u32,
        initial_health: u32,
        ir_params: IRParams,
    ) -> Result<(), Error> {
        initialize(
            &env,
            &admin,
            &treasury,
            flash_loan_fee,
            initial_health,
            &ir_params,
        )
    }

    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
        upgrade(&env, &new_wasm_hash)
    }

    fn upgrade_s_token(env: Env, asset: Address, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
        upgrade_s_token(&env, &asset, &new_wasm_hash)
    }

    fn upgrade_debt_token(
        env: Env,
        asset: Address,
        new_wasm_hash: BytesN<32>,
    ) -> Result<(), Error> {
        upgrade_debt_token(&env, &asset, &new_wasm_hash)
    }

    fn version() -> u32 {
        1
    }

    fn init_reserve(env: Env, asset: Address, reserve_type: ReserveType) -> Result<(), Error> {
        init_reserve(&env, &asset, reserve_type)
    }

    fn set_reserve_status(env: Env, asset: Address, is_active: bool) -> Result<(), Error> {
        set_reserve_status(&env, &asset, is_active)
    }

    fn set_ir_params(env: Env, input: IRParams) -> Result<(), Error> {
        set_ir_params(&env, &input)
    }

    fn reserve_timestamp_window(env: Env) -> u64 {
        read_reserve_timestamp_window(&env)
    }

    fn set_reserve_timestamp_window(env: Env, window: u64) -> Result<(), Error> {
        set_reserve_timestamp_window(&env, window)
    }

    fn ir_params(env: Env) -> Option<IRParams> {
        read_ir_params(&env).ok()
    }

    fn enable_borrowing_on_reserve(env: Env, asset: Address, enabled: bool) -> Result<(), Error> {
        enable_borrowing_on_reserve(&env, &asset, enabled)
    }

    fn configure_as_collateral(
        env: Env,
        asset: Address,
        params: CollateralParamsInput,
    ) -> Result<(), Error> {
        configure_as_collateral(&env, &asset, &params)
    }

    fn get_reserve(env: Env, asset: Address) -> Option<ReserveData> {
        read_reserve(&env, &asset).ok()
    }

    fn collat_coeff(env: Env, asset: Address) -> Result<i128, Error> {
        collat_coeff(&env, &asset)
    }

    fn debt_coeff(env: Env, asset: Address) -> Result<i128, Error> {
        debt_coeff(&env, &asset)
    }

    fn base_asset(env: Env) -> Result<BaseAssetConfig, Error> {
        read_base_asset(&env)
    }

    fn set_base_asset(env: Env, asset: Address, decimals: u32) -> Result<(), Error> {
        set_base_asset(&env, &asset, decimals)
    }

    fn initial_health(env: Env) -> Result<u32, Error> {
        read_initial_health(&env)
    }

    fn set_initial_health(env: Env, value: u32) -> Result<(), Error> {
        set_initial_health(&env, value)
    }

    fn set_price_feeds(env: Env, inputs: Vec<PriceFeedConfigInput>) -> Result<(), Error> {
        set_price_feeds(&env, &inputs)
    }

    fn price_feeds(env: Env, asset: Address) -> Option<PriceFeedConfig> {
        read_price_feeds(&env, &asset).ok()
    }

    fn deposit(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error> {
        deposit(&env, &who, &asset, amount)
    }

    fn repay(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error> {
        repay(&env, &who, &asset, amount)
    }

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

    fn withdraw(
        env: Env,
        who: Address,
        asset: Address,
        amount: i128,
        to: Address,
    ) -> Result<(), Error> {
        withdraw(&env, &who, &asset, amount, &to)
    }

    fn borrow(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error> {
        borrow(&env, &who, &asset, amount)
    }

    fn set_pause(env: Env, value: bool) -> Result<(), Error> {
        set_pause(&env, value)
    }

    fn paused(env: Env) -> bool {
        paused(&env)
    }

    fn treasury(e: Env) -> Address {
        read_treasury(&e)
    }

    fn account_position(env: Env, who: Address) -> Result<AccountPosition, Error> {
        account_position(&env, &who)
    }

    fn liquidate(
        env: Env,
        liquidator: Address,
        who: Address,
        receive_stoken: bool,
    ) -> Result<(), Error> {
        liquidate(&env, &liquidator, &who, receive_stoken)
    }

    fn set_as_collateral(
        env: Env,
        who: Address,
        asset: Address,
        use_as_collateral: bool,
    ) -> Result<(), Error> {
        set_as_collateral(&env, &who, &asset, use_as_collateral)
    }

    fn user_configuration(env: Env, who: Address) -> Result<UserConfiguration, Error> {
        read_user_config(&env, &who)
    }

    fn stoken_underlying_balance(env: Env, stoken_address: Address) -> i128 {
        read_stoken_underlying_balance(&env, &stoken_address)
    }

    fn token_balance(env: Env, token: Address, account: Address) -> i128 {
        read_token_balance(&env, &token, &account)
    }

    fn token_total_supply(env: Env, token: Address) -> i128 {
        read_token_total_supply(&env, &token)
    }

    fn set_flash_loan_fee(env: Env, fee: u32) -> Result<(), Error> {
        set_flash_loan_fee(&env, fee)
    }

    fn flash_loan_fee(env: Env) -> u32 {
        read_flash_loan_fee(&env)
    }

    fn flash_loan(
        env: Env,
        who: Address,
        receiver: Address,
        loan_assets: Vec<FlashLoanAsset>,
        params: Bytes,
    ) -> Result<(), Error> {
        flash_loan(&env, &who, &receiver, &loan_assets, &params)
    }

    fn twap_median_price(env: Env, asset: Address, amount: i128) -> Result<i128, Error> {
        twap_median_price(env, asset, amount)
    }

    fn balance(env: Env, id: Address, asset: Address) -> i128 {
        read_token_balance(&env, &asset, &id)
    }
}
