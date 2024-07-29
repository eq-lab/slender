use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 0,
    Uninitialized = 1,
    Paused = 2,
    BellowMinValue = 3,
    ExceededMaxValue = 4,
    GracePeriod = 5,

    NoActiveReserve = 100,
    ReservesMaxCapacityExceeded = 101,
    NoPriceForAsset = 102,
    InvalidAssetPrice = 103,
    LiquidationOrderMustBeUnique = 104,
    NotFungible = 105,

    NotEnoughAvailableUserBalance = 200,
    DebtError = 201,

    BorrowingDisabled = 300,
    GoodPosition = 301,
    InvalidAmount = 302,
    ValidateBorrowMathError = 303,
    CalcAccountDataMathError = 304,
    LiquidateMathError = 305,
    MustNotBeInCollateralAsset = 306,
    FlashLoanReceiverError = 307,

    MathOverflowError = 400,
    MustBeLtePercentageFactor = 401,
    MustBeLtPercentageFactor = 402,
    MustBeGtPercentageFactor = 403,
    MustBeNonNegative = 404,

    AccruedRateMathError = 500,
    CollateralCoeffMathError = 501,
    DebtCoeffMathError = 502,
}
