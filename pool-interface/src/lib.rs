#![deny(warnings)]
#![no_std]

pub use reserve_config::*;
use soroban_sdk::{
    contractclient, contracterror, contractspecfn, contracttype, Address, BytesN, Env, Vec,
};
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
    InvalidAssetPrice = 106,

    UserConfigInvalidIndex = 200,
    NotEnoughAvailableUserBalance = 201,
    UserConfigNotExists = 202,
    MustHaveDebt = 203,
    MustNotHaveDebt = 204,

    BorrowingNotEnabled = 300,
    CollateralNotCoverNewBorrow = 301,
    BadPosition = 302,
    GoodPosition = 303,
    InvalidAmount = 304,
    ValidateBorrowMathError = 305,
    CalcAccountDataMathError = 306,
    AssetPriceMathError = 307,
    NotEnoughCollateral = 308,
    LiquidateMathError = 309,
    MustNotBeInCollateralAsset = 310,
    UtilizationCapExceeded = 311,
    LiqCapExceeded = 312,

    MathOverflowError = 400,
    MustBeLtePercentageFactor = 401,
    MustBeLtPercentageFactor = 402,
    MustBeGtPercentageFactor = 403,
    MustBePositive = 404,

    AccruedRateMathError = 500,
    CollateralCoeffMathError = 501,
    DebtCoeffMathError = 502,
}

#[contracttype]
pub struct AccountPosition {
    pub discounted_collateral: i128,
    pub debt: i128,
    pub npv: i128,
}

#[contracttype]
pub struct AssetBalance {
    pub asset: Address,
    pub balance: i128,
}

impl AssetBalance {
    pub fn new(asset: Address, balance: i128) -> Self {
        Self { asset, balance }
    }
}

#[cfg(feature = "exceeded-limit-fix")]
#[contracttype]
pub struct MintBurn {
    pub asset_balance: AssetBalance,
    pub mint: bool,
    pub who: Address,
}

#[cfg(feature = "exceeded-limit-fix")]
impl MintBurn {
    pub fn new(asset_balance: AssetBalance, mint: bool, who: Address) -> Self {
        Self {
            asset_balance,
            mint,
            who,
        }
    }
}

/// Interface for SToken
#[contractspecfn(name = "Spec", export = false)]
#[contractclient(name = "LendingPoolClient")]
pub trait LendingPoolTrait {
    fn initialize(
        env: Env,
        admin: Address,
        treasury: Address,
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
}
