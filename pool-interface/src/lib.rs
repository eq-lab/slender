#![deny(warnings)]
#![no_std]

pub use reserve_config::*;
use soroban_sdk::{contractclient, contracterror, contractspecfn, Address, Env};
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
    ReserveAlreadyInitialized = 2,

    NoReserveExistForAsset = 3,
    InvalidAmount = 4,
    NoActiveReserve = 5,
    ReserveFrozen = 6,

    UserConfigInvalidIndex = 7,
    NotEnoughAvailableUserBalance = 8,
    UserConfigNotExists = 9,
    MathOverflowError = 10,

    BorrowingNotEnabled = 11,
    HealthFactorLowerThanLiqThreshold = 12,
    CollateralIsZero = 13,
    CollateralSameAsBorrow = 14,
    CollateralNotCoverNewBorrow = 15,

    InvalidReserveParams = 16,
    ReserveLiquidityNotZero = 17,

    ValidateBorrowMathError = 18,
    CalcAccountDataMathError = 19,

    ReservesMaxCapacityExceeded = 20,
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
        config: CollateralParams,
    ) -> Result<(), Error>;

    fn get_reserve(env: Env, asset: Address) -> Option<ReserveData>;

    fn deposit(env: Env, who: Address, asset: Address, amount: i128) -> Result<(), Error>;

    fn finalize_transfer(
        _asset: Address,
        _from: Address,
        _to: Address,
        _amount: i128,
        _balance_from_before: i128,
        _balance_to_before: i128,
    );

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
}
