use soroban_sdk::contracttype;

#[contracttype]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Permission {
    ClaimProtocolFee,
    CollateralReserveParams,
    SetReserveBorrowing,
    InitReserve,
    SetGracePeriod,
    SetIRParams,
    SetPause,
    SetPoolConfiguration,
    SetPriceFeeds,
    SetReserveStatus,
    UpgradeLPTokens,
    UpgradePoolWasm,
    Permission,
}
