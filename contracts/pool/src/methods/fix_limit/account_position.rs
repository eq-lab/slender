use common::FixedI128;
use pool_interface::types::account_position::AccountPosition;
use pool_interface::types::asset_balance::AssetBalance;
use pool_interface::types::error::Error;
use pool_interface::types::user_config::UserConfiguration;
use soroban_sdk::{assert_with_error, vec, Address, Env, Map, Vec};

use crate::methods::utils::get_collat_coeff::get_collat_coeff;
use crate::methods::utils::rate::get_actual_borrower_accrued_rate;
use crate::storage::{
    read_price, read_reserve, read_reserves, read_token_balance, read_token_total_supply,
    read_user_config,
};
use crate::types::account_data::AccountData;
use crate::types::liquidation_collateral::LiquidationCollateral;
use crate::types::liquidation_data::LiquidationData;

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

pub fn account_position(env: &Env, who: &Address) -> Result<AccountPosition, Error> {
    let user_config = read_user_config(env, who)?;
    let account_data =
        calc_account_data(env, who, CalcAccountDataCache::none(), &user_config, false)?;

    Ok(account_data.get_position())
}

#[allow(clippy::too_many_arguments)]
pub fn calc_account_data(
    env: &Env,
    who: &Address,
    cache: CalcAccountDataCache,
    user_config: &UserConfiguration,
    liquidation: bool,
) -> Result<AccountData, Error> {
    if user_config.is_empty() {
        return Ok(AccountData::default(env, liquidation));
    }

    let CalcAccountDataCache {
        mb_who_collat,
        mb_who_debt,
        mb_s_token_supply,
        mb_debt_token_supply,
    } = cache;

    let mut total_discounted_collateral_in_xlm: i128 = 0;
    let mut total_debt_in_xlm: i128 = 0;
    let mut total_debt_with_penalty_in_xlm: i128 = 0;
    let mut debt_to_cover = Vec::new(env);
    let mut sorted_collateral_to_receive = Map::new(env);
    let reserves = read_reserves(env);
    let reserves_len =
        u8::try_from(reserves.len()).map_err(|_| Error::ReservesMaxCapacityExceeded)?;

    // calc collateral and debt expressed in XLM token
    for i in 0..reserves_len {
        if !user_config.is_using_as_collateral_or_borrowing(env, i) {
            continue;
        }

        let curr_reserve_asset = reserves.get_unchecked(i.into());
        let curr_reserve = read_reserve(env, &curr_reserve_asset)?;

        assert_with_error!(
            env,
            curr_reserve.configuration.is_active || !liquidation,
            Error::NoActiveReserve
        );

        let asset_price = FixedI128::from_inner(read_price(env, &curr_reserve_asset));

        if user_config.is_using_as_collateral(env, i) {
            let s_token_supply = mb_s_token_supply
                .filter(|x| x.asset == curr_reserve.s_token_address)
                .map(|x| x.balance)
                .unwrap_or_else(|| read_token_total_supply(env, &curr_reserve.s_token_address));
            let debt_token_supply = mb_debt_token_supply
                .filter(|x| x.asset == curr_reserve.debt_token_address)
                .map(|x| x.balance)
                .unwrap_or_else(|| read_token_total_supply(env, &curr_reserve.debt_token_address));

            let collat_coeff =
                get_collat_coeff(env, &curr_reserve, s_token_supply, debt_token_supply)?;

            let who_collat = mb_who_collat
                .filter(|x| x.asset == curr_reserve.s_token_address)
                .map(|x| x.balance)
                .unwrap_or_else(|| read_token_balance(env, &curr_reserve.s_token_address, who));

            let discount = FixedI128::from_percentage(curr_reserve.configuration.discount)
                .ok_or(Error::CalcAccountDataMathError)?;

            let compounded_balance = collat_coeff
                .mul_int(who_collat)
                .ok_or(Error::CalcAccountDataMathError)?;

            let compounded_balance_in_xlm = asset_price
                .mul_int(compounded_balance)
                .ok_or(Error::CalcAccountDataMathError)?;

            let discounted_balance_in_xlm = discount
                .mul_int(compounded_balance_in_xlm)
                .ok_or(Error::CalcAccountDataMathError)?;

            total_discounted_collateral_in_xlm = total_discounted_collateral_in_xlm
                .checked_add(discounted_balance_in_xlm)
                .ok_or(Error::CalcAccountDataMathError)?;

            if liquidation {
                let curr_discount = curr_reserve.configuration.discount;
                let mut collateral_to_receive = sorted_collateral_to_receive
                    .get(curr_discount)
                    .unwrap_or(Vec::new(env));
                collateral_to_receive.push_back(LiquidationCollateral {
                    reserve_data: curr_reserve,
                    asset: curr_reserve_asset,
                    s_token_balance: who_collat,
                    asset_price: asset_price.into_inner(),
                    collat_coeff: collat_coeff.into_inner(),
                });
                sorted_collateral_to_receive.set(curr_discount, collateral_to_receive);
            }
        } else if user_config.is_borrowing(env, i) {
            let debt_coeff = get_actual_borrower_accrued_rate(env, &curr_reserve)?;

            let who_debt = mb_who_debt
                .filter(|x| x.asset == curr_reserve.debt_token_address)
                .map(|x| x.balance)
                .unwrap_or_else(|| read_token_balance(env, &curr_reserve.debt_token_address, who));

            let compounded_balance = debt_coeff
                .mul_int(who_debt)
                .ok_or(Error::CalcAccountDataMathError)?;

            let debt_balance_in_xlm = asset_price
                .mul_int(compounded_balance)
                .ok_or(Error::CalcAccountDataMathError)?;

            total_debt_in_xlm = total_debt_in_xlm
                .checked_add(debt_balance_in_xlm)
                .ok_or(Error::CalcAccountDataMathError)?;

            if liquidation {
                let liq_bonus = FixedI128::from_percentage(curr_reserve.configuration.liq_bonus)
                    .ok_or(Error::CalcAccountDataMathError)?;
                let liquidation_debt = liq_bonus
                    .mul_int(debt_balance_in_xlm)
                    .ok_or(Error::CalcAccountDataMathError)?;
                total_debt_with_penalty_in_xlm = total_debt_with_penalty_in_xlm
                    .checked_add(liquidation_debt)
                    .ok_or(Error::CalcAccountDataMathError)?;

                debt_to_cover.push_back((
                    curr_reserve_asset,
                    curr_reserve,
                    compounded_balance,
                    who_debt,
                ));
            }
        }
    }

    let npv = total_discounted_collateral_in_xlm
        .checked_sub(total_debt_in_xlm)
        .ok_or(Error::CalcAccountDataMathError)?;

    let liquidation_data = || -> LiquidationData {
        let mut collateral_to_receive = vec![env];
        let sorted = sorted_collateral_to_receive.values();
        for v in sorted {
            for c in v {
                collateral_to_receive.push_back(c);
            }
        }

        LiquidationData {
            total_debt_with_penalty_in_xlm,
            debt_to_cover,
            collateral_to_receive,
        }
    };

    Ok(AccountData {
        discounted_collateral: total_discounted_collateral_in_xlm,
        debt: total_debt_in_xlm,
        liquidation: liquidation.then_some(liquidation_data()),
        npv,
    })
}
