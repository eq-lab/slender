use pool_interface::types::asset_balance::AssetBalance;

pub struct CalcAccountDataCache<'a> {
    pub mb_who_collat: Option<&'a AssetBalance>,
    pub mb_who_debt: Option<&'a AssetBalance>,
    pub mb_s_token_supply: Option<&'a AssetBalance>,
    pub mb_debt_token_supply: Option<&'a AssetBalance>,
}

impl<'a> CalcAccountDataCache<'a> {
    pub fn none() -> Self {
        Self {
            mb_who_collat: None,
            mb_who_debt: None,
            mb_s_token_supply: None,
            mb_debt_token_supply: None,
        }
    }
}
