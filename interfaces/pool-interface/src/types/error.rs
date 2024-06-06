use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 0,
    Uninitialized = 1,
    Paused = 2,
    BellowMinValue = 3,
    AboveMaxValue = 4,
    GracePeriod = 6,

    NoActiveReserve = 100,
    ReservesMaxCapacityExceeded = 103,
    NoPriceForAsset = 104,
    InvalidAssetPrice = 105,
    LiquidationOrderMustBeUnique = 108,
    NotFungible = 109,
    ExceededMaxValue = 110,

    NotEnoughAvailableUserBalance = 200,
    DebtError = 203,

    BorrowingDisabled = 300,
    GoodPosition = 302,
    InvalidAmount = 303,
    ValidateBorrowMathError = 304,
    CalcAccountDataMathError = 305,
    LiquidateMathError = 306,
    MustNotBeInCollateralAsset = 307,
    FlashLoanReceiverError = 310,

    MathOverflowError = 400,
    MustBeLtePercentageFactor = 401,
    MustBeLtPercentageFactor = 402,
    MustBeGtPercentageFactor = 403,
    MustBeNonNegative = 405,

    AccruedRateMathError = 500,
    CollateralCoeffMathError = 501,
    DebtCoeffMathError = 502,
}
