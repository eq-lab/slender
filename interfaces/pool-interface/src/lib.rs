#![deny(warnings)]
#![no_std]

use soroban_sdk::{contractclient, contractspecfn, Address, Bytes, BytesN, Env, Vec};
#[cfg(feature = "exceeded-limit-fix")]
use types::mint_burn::MintBurn;
use types::{
    account_position::AccountPosition, collateral_params_input::CollateralParamsInput,
    error::Error, flash_loan_asset::FlashLoanAsset, init_reserve_input::InitReserveInput,
    ir_params::IRParams, reserve_data::ReserveData, user_config::UserConfiguration,
};

pub mod types;

pub struct Spec;

/// Interface for SToken
#[contractspecfn(name = "Spec", export = false)]
#[contractclient(name = "LendingPoolClient")]
pub trait LendingPoolTrait {
    fn initialize(
        env: Env,
        admin: Address,
        treasury: Address,
        flash_loan_fee: u32,
        ir_params: IRParams,
    ) -> Result<(), Error>;

    fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error>;

    fn upgrade_s_token(env: Env, asset: Address, new_wasm_hash: BytesN<32>) -> Result<(), Error>;

    fn upgrade_debt_token(env: Env, asset: Address, new_wasm_hash: BytesN<32>)
        -> Result<(), Error>;

    fn version() -> u32;

    fn init_reserve(env: Env, asset: Address, input: InitReserveInput) -> Result<(), Error>;

    fn set_reserve_status(env: Env, asset: Address, is_active: bool) -> Result<(), Error>;

    fn configure_as_collateral(
        env: Env,
        asset: Address,
        config: CollateralParamsInput,
    ) -> Result<(), Error>;

    fn enable_borrowing_on_reserve(env: Env, asset: Address, enabled: bool) -> Result<(), Error>;

    fn get_reserve(env: Env, asset: Address) -> Option<ReserveData>;

    fn collat_coeff(env: Env, asset: Address) -> Result<i128, Error>;

    fn debt_coeff(env: Env, asset: Address) -> Result<i128, Error>;

    fn set_price_feed(env: Env, feed: Address, assets: Vec<Address>) -> Result<(), Error>;

    fn price_feed(env: Env, asset: Address) -> Option<Address>;

    fn set_ir_params(env: Env, input: IRParams) -> Result<(), Error>;

    fn ir_params(env: Env) -> Option<IRParams>;

    #[cfg(feature = "exceeded-limit-fix")]
    fn deposit(
        env: Env,
        who: Address,
        asset: Address,
        amount: i128,
    ) -> Result<Vec<MintBurn>, Error>;

    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn deposit(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error>;

    #[cfg(feature = "exceeded-limit-fix")]
    fn repay(env: Env, who: Address, asset: Address, amount: i128) -> Result<Vec<MintBurn>, Error>;

    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn repay(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error>;

    #[allow(clippy::too_many_arguments)]
    fn finalize_transfer(
        env: Env,
        asset: Address,
        from: Address,
        to: Address,
        amount: i128,
        balance_from_before: i128,
        balance_to_before: i128,
        total_supply: i128,
    ) -> Result<(), Error>;

    #[cfg(feature = "exceeded-limit-fix")]
    fn withdraw(
        env: Env,
        who: Address,
        asset: Address,
        amount: i128,
        to: Address,
    ) -> Result<Vec<MintBurn>, Error>;

    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn withdraw(
        env: Env,
        who: Address,
        asset: Address,
        amount: i128,
        to: Address,
    ) -> Result<(), Error>;

    fn stoken_underlying_balance(env: Env, stoken_address: Address) -> i128;

    #[cfg(feature = "exceeded-limit-fix")]
    fn borrow(env: Env, who: Address, asset: Address, amount: i128)
        -> Result<Vec<MintBurn>, Error>;

    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn borrow(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error>;

    fn set_pause(env: Env, value: bool) -> Result<(), Error>;

    fn paused(env: Env) -> bool;

    fn treasury(e: Env) -> Address;

    fn account_position(env: Env, who: Address) -> Result<AccountPosition, Error>;

    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn liquidate(
        env: Env,
        liquidator: Address,
        who: Address,
        receive_stoken: bool,
    ) -> Result<(), Error>;

    #[cfg(feature = "exceeded-limit-fix")]
    fn liquidate(
        env: Env,
        liquidator: Address,
        who: Address,
        receive_stoken: bool,
    ) -> Result<Vec<MintBurn>, Error>;

    fn set_as_collateral(
        env: Env,
        who: Address,
        asset: Address,
        use_as_collateral: bool,
    ) -> Result<(), Error>;

    fn user_configuration(env: Env, who: Address) -> Result<UserConfiguration, Error>;

    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn set_price(env: Env, asset: Address, price: i128);

    #[cfg(feature = "exceeded-limit-fix")]
    fn set_price(env: Env, asset: Address, price: i128);

    fn set_flash_loan_fee(env: Env, fee: u32) -> Result<(), Error>;

    fn flash_loan_fee(env: Env) -> u32;

    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn flash_loan(
        env: Env,
        who: Address,
        receiver: Address,
        assets: Vec<FlashLoanAsset>,
        params: Bytes,
    ) -> Result<(), Error>;

    #[cfg(feature = "exceeded-limit-fix")]
    fn flash_loan(
        env: Env,
        who: Address,
        receiver: Address,
        assets: Vec<FlashLoanAsset>,
        params: Bytes,
    ) -> Result<Vec<MintBurn>, Error>;

    #[cfg(feature = "exceeded-limit-fix")]
    fn get_price(env: Env, asset: Address) -> i128;

    #[cfg(not(feature = "exceeded-limit-fix"))]
    fn get_price(_env: Env, _asset: Address) -> i128;
}
