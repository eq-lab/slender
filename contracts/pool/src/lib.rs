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
    initialize::initialize, set_base_asset::set_base_asset, set_decimals::set_decimals,
    set_flash_loan_fee::set_flash_loan_fee, set_ir_params::set_ir_params, set_pause::set_pause,
    set_price_feed::set_price_feed, set_reserve_status::set_reserve_status,
    token_total_supply::token_total_supply, upgrade::upgrade,
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
    fn initialize(
        env: Env,
        admin: Address,
        treasury: Address,
        flash_loan_fee: u32,
        ir_params: IRParams,
    ) -> Result<(), Error> {
        initialize(&env, &admin, &treasury, flash_loan_fee, &ir_params)
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

    fn init_reserve(env: Env, asset: Address, input: InitReserveInput) -> Result<(), Error> {
        init_reserve(&env, &asset, &input)
    }

    fn set_decimals(env: Env, asset: Address, decimals: u32) -> Result<(), Error> {
        set_decimals(&env, &asset, decimals)
    }

    fn set_base_asset(env: Env, asset: Address, is_base: bool) -> Result<(), Error> {
        set_base_asset(&env, &asset, is_base)
    }

    fn set_reserve_status(env: Env, asset: Address, is_active: bool) -> Result<(), Error> {
        set_reserve_status(&env, &asset, is_active)
    }

    fn set_ir_params(env: Env, input: IRParams) -> Result<(), Error> {
        set_ir_params(&env, &input)
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

    fn set_price_feed(env: Env, feed: Address, assets: Vec<Address>) -> Result<(), Error> {
        set_price_feed(&env, &feed, &assets)
    }

    fn price_feed(env: Env, asset: Address) -> Option<Address> {
        read_price_feed(&env, &asset).ok()
    }

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

    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn repay(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error> {
        repay(&env, &who, &asset, amount)
    }

    #[cfg(feature = "exceeded-limit-fix")]
    fn repay(env: Env, who: Address, asset: Address, amount: i128) -> Result<Vec<MintBurn>, Error> {
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

    fn treasury(e: Env) -> Address {
        read_treasury(&e)
    }

    fn account_position(env: Env, who: Address) -> Result<AccountPosition, Error> {
        account_position(&env, &who)
    }

    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn liquidate(
        env: Env,
        liquidator: Address,
        who: Address,
        receive_stoken: bool,
    ) -> Result<(), Error> {
        liquidate(&env, &liquidator, &who, receive_stoken)
    }

    #[cfg(feature = "exceeded-limit-fix")]
    fn liquidate(
        env: Env,
        liquidator: Address,
        who: Address,
        receive_stoken: bool,
    ) -> Result<Vec<MintBurn>, Error> {
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
        token_total_supply(&env, &token)
    }

    #[cfg(feature = "exceeded-limit-fix")]
    fn set_price(env: Env, asset: Address, price: i128) {
        write_price(&env, &asset, price);
    }

    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn set_price(_env: Env, _asset: Address, _price: i128) {
        unimplemented!()
    }

    fn set_flash_loan_fee(env: Env, fee: u32) -> Result<(), Error> {
        set_flash_loan_fee(&env, fee)
    }

    fn flash_loan_fee(env: Env) -> u32 {
        read_flash_loan_fee(&env)
    }

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
