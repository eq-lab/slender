#![deny(warnings)]
#![no_std]

use methods::revoke_permission::revoke_permission;
use methods::{
    account_position::account_position, borrow::borrow, claim_protocol_fee::claim_protocol_fee,
    collat_coeff::collat_coeff, configure_as_collateral::configure_as_collateral,
    debt_coeff::debt_coeff, deposit::deposit,
    enable_borrowing_on_reserve::enable_borrowing_on_reserve, finalize_transfer::finalize_transfer,
    flash_loan::flash_loan, grant_permission::grant_permission, init_reserve::init_reserve,
    initialize::initialize, liquidate::liquidate, pool_configuration::pool_configuration,
    repay::repay, set_as_collateral::set_as_collateral, set_ir_params::set_ir_params,
    set_pause::set_pause, set_pool_configuration::set_pool_configuration,
    set_price_feeds::set_price_feeds, set_reserve_status::set_reserve_status,
    twap_median_price::twap_median_price, upgrade::upgrade, upgrade_debt_token::upgrade_debt_token,
    upgrade_s_token::upgrade_s_token, withdraw::withdraw,
};
use pool_interface::types::permission::Permission;
use pool_interface::types::{
    account_position::AccountPosition, collateral_params_input::CollateralParamsInput,
    error::Error, flash_loan_asset::FlashLoanAsset, ir_params::IRParams, pause_info::PauseInfo,
    pool_config::PoolConfig, price_feed_config::PriceFeedConfig,
    price_feed_config_input::PriceFeedConfigInput, reserve_data::ReserveData,
    reserve_type::ReserveType, user_config::UserConfiguration,
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
        permisssions_owner: Address,
        ir_params: IRParams,
        pool_config: PoolConfig,
    ) -> Result<(), Error> {
        initialize(&env, &permisssions_owner, &ir_params, &pool_config)
    }

    fn upgrade(env: Env, who: Address, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
        upgrade(&env, &who, &new_wasm_hash)
    }

    fn upgrade_s_token(
        env: Env,
        who: Address,
        asset: Address,
        new_wasm_hash: BytesN<32>,
    ) -> Result<(), Error> {
        upgrade_s_token(&env, &who, &asset, &new_wasm_hash)
    }

    fn upgrade_debt_token(
        env: Env,
        who: Address,
        asset: Address,
        new_wasm_hash: BytesN<32>,
    ) -> Result<(), Error> {
        upgrade_debt_token(&env, &who, &asset, &new_wasm_hash)
    }

    fn version() -> u32 {
        1
    }

    fn init_reserve(
        env: Env,
        who: Address,
        asset: Address,
        reserve_type: ReserveType,
    ) -> Result<(), Error> {
        init_reserve(&env, &who, &asset, reserve_type)
    }

    fn set_reserve_status(
        env: Env,
        who: Address,
        asset: Address,
        is_active: bool,
    ) -> Result<(), Error> {
        set_reserve_status(&env, &who, &asset, is_active)
    }

    fn set_ir_params(env: Env, who: Address, input: IRParams) -> Result<(), Error> {
        set_ir_params(&env, Some(who), &input)
    }

    fn ir_params(env: Env) -> Option<IRParams> {
        read_ir_params(&env).ok()
    }

    fn enable_borrowing_on_reserve(
        env: Env,
        who: Address,
        asset: Address,
        enabled: bool,
    ) -> Result<(), Error> {
        enable_borrowing_on_reserve(&env, &who, &asset, enabled)
    }

    fn configure_as_collateral(
        env: Env,
        who: Address,
        asset: Address,
        params: CollateralParamsInput,
    ) -> Result<(), Error> {
        configure_as_collateral(&env, &who, &asset, &params)
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

    fn set_pool_configuration(env: Env, who: Address, config: PoolConfig) -> Result<(), Error> {
        set_pool_configuration(&env, Some(who), &config)
    }

    fn pool_configuration(env: Env) -> Result<PoolConfig, Error> {
        pool_configuration(&env)
    }

    fn set_price_feeds(
        env: Env,
        who: Address,
        inputs: Vec<PriceFeedConfigInput>,
    ) -> Result<(), Error> {
        set_price_feeds(&env, &who, &inputs)
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

    fn set_pause(env: Env, who: Address, value: bool) -> Result<(), Error> {
        set_pause(&env, &who, value)
    }

    fn pause_info(env: Env) -> Result<PauseInfo, Error> {
        read_pause_info(&env)
    }

    fn account_position(env: Env, who: Address) -> Result<AccountPosition, Error> {
        account_position(&env, &who)
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

    fn stoken_underlying_balance(env: Env, stoken_address: Address) -> i128 {
        read_stoken_underlying_balance(&env, &stoken_address)
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

    fn balance(env: Env, id: Address, asset: Address) -> i128 {
        read_token_balance(&env, &asset, &id)
    }

    fn protocol_fee(env: Env, asset: Address) -> i128 {
        read_protocol_fee_vault(&env, &asset)
    }

    fn claim_protocol_fee(
        env: Env,
        who: Address,
        asset: Address,
        recipient: Address,
    ) -> Result<(), Error> {
        claim_protocol_fee(&env, &who, &asset, &recipient)
    }

    fn grant_permission(
        env: Env,
        who: Address,
        receiver: Address,
        permission: Permission,
    ) -> Result<(), Error> {
        grant_permission(&env, &who, &receiver, &permission)
    }

    fn revoke_permission(
        env: Env,
        who: Address,
        owner: Address,
        permission: Permission,
    ) -> Result<(), Error> {
        revoke_permission(&env, &who, &owner, &permission)
    }

    fn permissioned(env: Env, permission: Permission) -> Vec<Address> {
        read_permission_owners(&env, &permission)
    }
}
