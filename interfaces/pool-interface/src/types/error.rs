use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 0,
    Uninitialized = 1,
    NoPriceFeed = 2,
    Paused = 3,
    NoPoolConfig = 4,
    ZeroGracePeriod = 5,
    GracePeriod = 6,
    NoPermissioned = 7,

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
    MustNotExceedAssetsLimit = 205,
    RequireMinBalance = 206,
    CollateralIsTooSmall = 207,
    DebtIsTooSmall = 208,

    BorrowingNotEnabled = 300,
    BelowInitialHealth = 301,
    GoodPosition = 302,
    InvalidAmount = 303,
    ValidateBorrowMathError = 304,
    CalcAccountDataMathError = 305,
    LiquidateMathError = 306,
    MustNotBeInCollateralAsset = 307,
    UtilizationCapExceeded = 308,
    LiqCapExceeded = 309,
    FlashLoanReceiverError = 310,

    MathOverflowError = 400,
    MustBeLtePercentageFactor = 401,
    MustBeLtPercentageFactor = 402,
    MustBeGtPercentageFactor = 403,
    MustBePositive = 404,
    MustBeNonNegative = 405,

    AccruedRateMathError = 500,
    CollateralCoeffMathError = 501,
    DebtCoeffMathError = 502,
}
