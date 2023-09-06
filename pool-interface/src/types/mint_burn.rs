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
