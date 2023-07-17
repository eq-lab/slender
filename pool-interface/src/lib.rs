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
    ReserveAlreadyInitialized = 3,
    Paused = 4,

    NoReserveExistForAsset = 5,
    InvalidAmount = 6,
    NoActiveReserve = 7,
    ReserveFrozen = 8,

    UserConfigInvalidIndex = 9,
    NotEnoughAvailableUserBalance = 10,
    UserConfigNotExists = 11,

    BorrowingNotEnabled = 12,
    HealthFactorLowerThanLiqThreshold = 13,
    CollateralNotCoverNewBorrow = 14,
    BadPosition = 15,

    InvalidReserveParams = 16,
    ReserveLiquidityNotZero = 17,

    ReservesMaxCapacityExceeded = 18,
    NoPriceForAsset = 19,

    MathOverflowError = 100,
    PriceMathOverflow = 101,
    ValidateBorrowMathError = 102,
    CalcAccountDataMathError = 103,
    CalcInterestRateMathError = 104,
    AssetPriceMathError = 105,
}

/// Interface for SToken
#[contractspecfn(name = "Spec", export = false)]
#[contractclient(name = "LendingPoolClient")]
pub trait LendingPoolTrait {
    fn initialize(env: Env, admin: Address) -> Result<(), Error>;

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

    fn deposit(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error>;

    fn finalize_transfer(
        env: Env,
        _asset: Address,
        _from: Address,
        _to: Address,
        _amount: i128,
        _balance_from_before: i128,
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
    fn set_liq_index(env: Env, asset: Address, value: i128) -> Result<(), Error>;

    fn borrow(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error>;

    fn set_pause(env: Env, value: bool) -> Result<(), Error>;

    fn paused(env: Env) -> bool;
}
