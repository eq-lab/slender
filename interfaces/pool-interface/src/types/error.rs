use soroban_sdk::contracterror;

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
    BaseAssetNotInitialized = 107,
    InitialHealthNotInitialized = 108,
    LiquidationOrderMustBeUnique = 109,
    NotFungible = 110,

    UserConfigInvalidIndex = 200,
    NotEnoughAvailableUserBalance = 201,
    UserConfigNotExists = 202,
    MustHaveDebt = 203,
    MustNotHaveDebt = 204,

    BorrowingNotEnabled = 300,
    BelowInitialHealth = 301,
    BadPosition = 302,
    GoodPosition = 303,
    InvalidAmount = 304,
    ValidateBorrowMathError = 305,
    CalcAccountDataMathError = 306,
    LiquidateMathError = 309,
    MustNotBeInCollateralAsset = 310,
    UtilizationCapExceeded = 311,
    LiqCapExceeded = 312,
    FlashLoanReceiverError = 313,

    MathOverflowError = 400,
    MustBeLtePercentageFactor = 401,
    MustBeLtPercentageFactor = 402,
    MustBeGtPercentageFactor = 403,
    MustBePositive = 404,

    AccruedRateMathError = 500,
    CollateralCoeffMathError = 501,
    DebtCoeffMathError = 502,
}
