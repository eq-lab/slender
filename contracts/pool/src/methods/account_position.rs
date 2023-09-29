use common::FixedI128;
use pool_interface::types::account_position::AccountPosition;
use pool_interface::types::error::Error;
use pool_interface::types::user_config::UserConfiguration;
use soroban_sdk::{assert_with_error, vec, Address, Env, Map, Vec};

use crate::storage::{
    read_reserve, read_reserves, read_token_balance, read_token_total_supply, read_user_config,
};
use crate::types::account_data::AccountData;
use crate::types::calc_account_data_cache::CalcAccountDataCache;
use crate::types::liquidation_collateral::LiquidationCollateral;
use crate::types::liquidation_data::LiquidationData;
use crate::types::price_provider::PriceProvider;

use super::utils::get_collat_coeff::get_collat_coeff;
use super::utils::rate::get_actual_borrower_accrued_rate;

pub fn account_position(env: &Env, who: &Address) -> Result<AccountPosition, Error> {
    let user_config = read_user_config(env, who)?;
    let account_data = calc_account_data(
        env,
        who,
        &CalcAccountDataCache::none(),
        &user_config,
        &mut PriceProvider::new(env)?,
        false,
    )?;

    Ok(account_data.get_position())
}

pub fn calc_account_data(
    env: &Env,
    who: &Address,
    cache: &CalcAccountDataCache,
    user_config: &UserConfiguration,
    price_provider: &mut PriceProvider,
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

    let mut total_discounted_collateral_in_base: i128 = 0;
    let mut total_debt_in_base: i128 = 0;
    let mut total_debt_with_penalty_in_base: i128 = 0;
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

        let asset = reserves.get_unchecked(i.into());
        let reserve = read_reserve(env, &asset)?;

        assert_with_error!(
            env,
            reserve.configuration.is_active || !liquidation,
            Error::NoActiveReserve
        );

        if user_config.is_using_as_collateral(env, i) {
            let s_token_supply = mb_s_token_supply
                .filter(|x| x.asset == reserve.s_token_address)
                .map(|x| x.balance)
                .unwrap_or_else(|| read_token_total_supply(env, &reserve.s_token_address));
            let debt_token_supply = mb_debt_token_supply
                .filter(|x| x.asset == reserve.debt_token_address)
                .map(|x| x.balance)
                .unwrap_or_else(|| read_token_total_supply(env, &reserve.debt_token_address));

            let collat_coeff = get_collat_coeff(env, &reserve, s_token_supply, debt_token_supply)?;

            let who_collat = mb_who_collat
                .filter(|x| x.asset == reserve.s_token_address)
                .map(|x| x.balance)
                .unwrap_or_else(|| read_token_balance(env, &reserve.s_token_address, who));

            let discount = FixedI128::from_percentage(reserve.configuration.discount)
                .ok_or(Error::CalcAccountDataMathError)?;

            let compounded_balance = collat_coeff
                .mul_int(who_collat)
                .ok_or(Error::CalcAccountDataMathError)?;

            let compounded_balance_in_base =
                price_provider.convert_to_base(&asset, compounded_balance)?;

            let discounted_balance_in_base = discount
                .mul_int(compounded_balance_in_base)
                .ok_or(Error::CalcAccountDataMathError)?;

            total_discounted_collateral_in_base = total_discounted_collateral_in_base
                .checked_add(discounted_balance_in_base)
                .ok_or(Error::CalcAccountDataMathError)?;

            if liquidation {
                let curr_discount = reserve.configuration.discount;
                let mut collateral_to_receive = sorted_collateral_to_receive
                    .get(curr_discount)
                    .unwrap_or(Vec::new(env));
                collateral_to_receive.push_back(LiquidationCollateral {
                    reserve_data: reserve,
                    asset,
                    s_token_balance: who_collat,
                    collat_coeff: collat_coeff.into_inner(),
                });
                sorted_collateral_to_receive.set(curr_discount, collateral_to_receive);
            }
        } else if user_config.is_borrowing(env, i) {
            let debt_coeff = get_actual_borrower_accrued_rate(env, &reserve)?;

            let who_debt = mb_who_debt
                .filter(|x| x.asset == reserve.debt_token_address)
                .map(|x| x.balance)
                .unwrap_or_else(|| read_token_balance(env, &reserve.debt_token_address, who));

            let compounded_balance = debt_coeff
                .mul_int(who_debt)
                .ok_or(Error::CalcAccountDataMathError)?;

            let debt_balance_in_base =
                price_provider.convert_to_base(&asset, compounded_balance)?;

            total_debt_in_base = total_debt_in_base
                .checked_add(debt_balance_in_base)
                .ok_or(Error::CalcAccountDataMathError)?;

            if liquidation {
                let liq_bonus = FixedI128::from_percentage(reserve.configuration.liq_bonus)
                    .ok_or(Error::CalcAccountDataMathError)?;
                let liquidation_debt = liq_bonus
                    .mul_int(debt_balance_in_base)
                    .ok_or(Error::CalcAccountDataMathError)?;
                total_debt_with_penalty_in_base = total_debt_with_penalty_in_base
                    .checked_add(liquidation_debt)
                    .ok_or(Error::CalcAccountDataMathError)?;

                debt_to_cover.push_back((asset, reserve, compounded_balance, who_debt));
            }
        }
    }

    let npv = total_discounted_collateral_in_base
        .checked_sub(total_debt_in_base)
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
            total_debt_with_penalty_in_base,
            debt_to_cover,
            collateral_to_receive,
        }
    };

    Ok(AccountData {
        discounted_collateral: total_discounted_collateral_in_base,
        debt: total_debt_in_base,
        liquidation: liquidation.then_some(liquidation_data()),
        npv,
    })
}
