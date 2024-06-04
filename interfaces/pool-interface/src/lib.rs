#![deny(warnings)]
#![no_std]

use soroban_sdk::{contractclient, contractspecfn, Address, Bytes, BytesN, Env, Vec};
use types::account_position::AccountPosition;
use types::collateral_params_input::CollateralParamsInput;
use types::error::Error;
use types::flash_loan_asset::FlashLoanAsset;
use types::ir_params::IRParams;
use types::pause_info::PauseInfo;
use types::permission::Permission;
use types::pool_config::PoolConfig;
use types::price_feed_config::PriceFeedConfig;
use types::price_feed_config_input::PriceFeedConfigInput;
use types::reserve_data::ReserveData;
use types::reserve_type::ReserveType;
use types::user_config::UserConfiguration;

pub mod types;

pub struct Spec;

/// Interface for SToken
#[contractspecfn(name = "Spec", export = false)]
#[contractclient(name = "LendingPoolClient")]
pub trait LendingPoolTrait {
    fn initialize(
        env: Env,
        admin: Address,
        flash_loan_fee: u32,
        initial_health: u32,
        ir_params: IRParams,
        grace_period: u64,
    ) -> Result<(), Error>;

    fn upgrade(env: Env, who: Address, new_wasm_hash: BytesN<32>) -> Result<(), Error>;

    fn upgrade_s_token(
        env: Env,
        who: Address,
        asset: Address,
        new_wasm_hash: BytesN<32>,
    ) -> Result<(), Error>;

    fn upgrade_debt_token(
        env: Env,
        who: Address,
        asset: Address,
        new_wasm_hash: BytesN<32>,
    ) -> Result<(), Error>;

    fn version() -> u32;

    fn init_reserve(
        env: Env,
        who: Address,
        asset: Address,
        reserve_type: ReserveType,
    ) -> Result<(), Error>;

    fn set_reserve_status(
        env: Env,
        who: Address,
        asset: Address,
        is_active: bool,
    ) -> Result<(), Error>;

    fn configure_as_collateral(
        env: Env,
        who: Address,
        asset: Address,
        config: CollateralParamsInput,
    ) -> Result<(), Error>;

    fn enable_borrowing_on_reserve(
        env: Env,
        who: Address,
        asset: Address,
        enabled: bool,
    ) -> Result<(), Error>;

    fn get_reserve(env: Env, asset: Address) -> Option<ReserveData>;

    fn collat_coeff(env: Env, asset: Address) -> Result<i128, Error>;

    fn debt_coeff(env: Env, asset: Address) -> Result<i128, Error>;

    fn set_pool_configuration(env: Env, who: Address, config: PoolConfig) -> Result<(), Error>;

    fn pool_configuration(env: Env) -> Result<PoolConfig, Error>;

    fn set_price_feeds(
        env: Env,
        who: Address,
        inputs: Vec<PriceFeedConfigInput>,
    ) -> Result<(), Error>;

    fn price_feeds(env: Env, asset: Address) -> Option<PriceFeedConfig>;

    fn set_ir_params(env: Env, who: Address, input: IRParams) -> Result<(), Error>;

    fn ir_params(env: Env) -> Option<IRParams>;

    fn deposit(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error>;

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

    fn withdraw(
        env: Env,
        who: Address,
        asset: Address,
        amount: i128,
        to: Address,
    ) -> Result<(), Error>;

    fn stoken_underlying_balance(env: Env, stoken_address: Address) -> i128;

    fn token_balance(env: Env, token: Address, account: Address) -> i128;

    fn token_total_supply(env: Env, token: Address) -> i128;

    fn borrow(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error>;

    fn set_pause(env: Env, who: Address, value: bool) -> Result<(), Error>;

    fn set_grace_period(env: Env, who: Address, grace_period: u64) -> Result<(), Error>;

    fn pause_info(env: Env) -> Result<PauseInfo, Error>;

    fn account_position(env: Env, who: Address) -> Result<AccountPosition, Error>;

    fn liquidate(env: Env, liquidator: Address, who: Address) -> Result<(), Error>;

    fn set_as_collateral(
        env: Env,
        who: Address,
        asset: Address,
        use_as_collateral: bool,
    ) -> Result<(), Error>;

    fn user_configuration(env: Env, who: Address) -> Result<UserConfiguration, Error>;

    fn flash_loan(
        env: Env,
        who: Address,
        receiver: Address,
        assets: Vec<FlashLoanAsset>,
        params: Bytes,
    ) -> Result<(), Error>;

    fn twap_median_price(env: Env, asset: Address, amount: i128) -> Result<i128, Error>;

    fn balance(env: Env, id: Address, asset: Address) -> i128;

    fn protocol_fee(env: Env, asset: Address) -> i128;

    fn claim_protocol_fee(
        env: Env,
        who: Address,
        asset: Address,
        recipient: Address,
    ) -> Result<(), Error>;

    fn grant_permission(
        env: Env,
        who: Address,
        receiver: Address,
        permission: Permission,
    ) -> Result<(), Error>;

    fn revoke_permission(
        env: Env,
        who: Address,
        owner: Address,
        permission: Permission,
    ) -> Result<(), Error>;

    fn permissioned(env: Env, permission: Permission) -> Vec<Address>;
}
