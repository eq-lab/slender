#![deny(warnings)]
#![no_std]

pub use reserve_config::*;
use soroban_sdk::{contractclient, contracterror, contractspecfn, Address, Env, Vec};
pub use user_config::*;

mod reserve_config;
mod user_config;

pub struct Spec;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 0,
    Uninitialized = 1,
    NoPriceFeed = 2,
    Paused = 3,

    NoReserveExistForAsset = 100,
    NoActiveReserve = 101,
    ReserveFrozen = 102,
    ReservesMaxCapacityExceeded = 103,
    NoPriceForAsset = 104,
    ReserveAlreadyInitialized = 105,

    UserConfigInvalidIndex = 200,
    NotEnoughAvailableUserBalance = 201,
    UserConfigNotExists = 202,

    BorrowingNotEnabled = 300,
    CollateralNotCoverNewBorrow = 301,
    BadPosition = 302,
    InvalidAmount = 303,
    ValidateBorrowMathError = 304,
    CalcAccountDataMathError = 305,
    AssetPriceMathError = 306,
    LiqCapExceeded = 307,

    MathOverflowError = 400,
    MustBeLtePercentageFactor = 401,
    MustBeLtPercentageFactor = 402,
    MustBeGtPercentageFactor = 403,
    MustBePositive = 404,

    AccruedRateMathError = 500,
    CollateralCoeffMathError = 501,
    DebtCoeffMathError = 502,
}

/// Interface for SToken
#[contractspecfn(name = "Spec", export = false)]
#[contractclient(name = "LendingPoolClient")]
pub trait LendingPoolTrait {
    fn initialize(env: Env, admin: Address, ir_params: IRParams) -> Result<(), Error>;

    fn init_reserve(env: Env, asset: Address, input: InitReserveInput) -> Result<(), Error>;

    fn configure_as_collateral(
        env: Env,
        asset: Address,
        config: CollateralParamsInput,
    ) -> Result<(), Error>;

    fn enable_borrowing_on_reserve(env: Env, asset: Address, enabled: bool) -> Result<(), Error>;

    fn get_reserve(env: Env, asset: Address) -> Option<ReserveData>;

    fn set_price_feed(env: Env, feed: Address, assets: Vec<Address>) -> Result<(), Error>;

    fn get_price_feed(env: Env, asset: Address) -> Option<Address>;

    fn set_ir_params(env: Env, input: IRParams) -> Result<(), Error>;

    fn get_ir_params(env: Env) -> Option<IRParams>;

    fn deposit(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error>;

    fn finalize_transfer(
        env: Env,
        asset: Address,
        from: Address,
        _to: Address,
        amount: i128,
        balance_from_before: i128,
        _balance_to_before: i128,
    ) -> Result<(), Error>;

    fn withdraw(
        env: Env,
        who: Address,
        asset: Address,
        amount: i128,
        to: Address,
    ) -> Result<(), Error>;

    #[cfg(any(test, feature = "testutils"))]
    fn set_accrued_rates(
        env: Env,
        asset: Address,
        collat_accrued_rate: Option<i128>,
        debt_accrued_rate: Option<i128>,
    ) -> Result<(), Error>;

    fn borrow(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error>;

    fn set_pause(env: Env, value: bool) -> Result<(), Error>;

    fn paused(env: Env) -> bool;
}
