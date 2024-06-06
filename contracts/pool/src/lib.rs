#![deny(warnings)]
#![no_std]

use methods::{
    account_position::account_position, borrow::borrow, claim_protocol_fee::claim_protocol_fee,
    collat_coeff::collat_coeff, configure_as_collateral::configure_as_collateral,
    debt_coeff::debt_coeff, deposit::deposit,
    enable_borrowing_on_reserve::enable_borrowing_on_reserve, finalize_transfer::finalize_transfer,
    flash_loan::flash_loan, init_reserve::init_reserve, initialize::initialize,
    liquidate::liquidate, repay::repay, set_as_collateral::set_as_collateral, set_pause::set_pause,
    set_pool_configuration::set_pool_configuration, set_price_feeds::set_price_feeds,
    set_reserve_status::set_reserve_status, twap_median_price::twap_median_price, upgrade::upgrade,
    upgrade_token::upgrade_token, withdraw::withdraw,
};
use pool_interface::types::{
    account_position::AccountPosition, collateral_params_input::CollateralParamsInput,
    error::Error, flash_loan_asset::FlashLoanAsset, pause_info::PauseInfo, pool_config::PoolConfig,
    price_feed_config::PriceFeedConfig, price_feed_config_input::PriceFeedConfigInput,
    reserve_data::ReserveData, reserve_type::ReserveType, user_config::UserConfiguration,
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
    fn initialize(env: Env, admin: Address, pool_config: PoolConfig) -> Result<(), Error> {
        initialize(&env, &admin, &pool_config)
    }

    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
        upgrade(&env, &new_wasm_hash)
    }

    fn upgrade_token(
        env: Env,
        asset: Address,
        s_token: bool,
        new_wasm_hash: BytesN<32>,
    ) -> Result<(), Error> {
        upgrade_token(&env, &asset, &new_wasm_hash, s_token)
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

    fn set_pool_configuration(env: Env, config: PoolConfig) -> Result<(), Error> {
        set_pool_configuration(&env, &config, true)
    }

    fn pool_configuration(env: Env) -> Result<PoolConfig, Error> {
        read_pool_config(&env)
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

    fn pause_info(env: Env) -> PauseInfo {
        read_pause_info(&env)
    }

    fn account_position(env: Env, who: Address) -> Result<AccountPosition, Error> {
        account_position(&env, &who, &read_pool_config(&env)?)
    }

    fn liquidate(env: Env, liquidator: Address, who: Address) -> Result<(), Error> {
        liquidate(&env, &liquidator, &who)
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

    fn token_balance(env: Env, token: Address, account: Address) -> i128 {
        read_token_balance(&env, &token, &account)
    }

    fn token_total_supply(env: Env, token: Address) -> i128 {
        read_token_total_supply(&env, &token)
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

    fn protocol_fee(env: Env, asset: Address) -> i128 {
        read_protocol_fee_vault(&env, &asset)
    }

    fn claim_protocol_fee(env: Env, asset: Address, recipient: Address) -> Result<(), Error> {
        claim_protocol_fee(&env, &asset, &recipient)
    }
}
