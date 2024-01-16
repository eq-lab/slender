use common::{FixedI128, PERCENTAGE_FACTOR};
use pool_interface::types::account_position::AccountPosition;
use pool_interface::types::error::Error;
use pool_interface::types::user_config::UserConfiguration;
use soroban_sdk::{assert_with_error, Address, Env, Map};

use crate::methods::utils::rate::calc_utilization;
use crate::storage::{
    read_initial_health, read_reserve, read_reserves, read_token_balance, read_token_total_supply,
    read_user_config,
};
use crate::types::account_data::AccountData;
use crate::types::calc_account_data_cache::CalcAccountDataCache;
use crate::types::liquidation_collateral::LiquidationCollateral;
use crate::types::liquidation_data::LiquidationData;
use crate::types::liquidation_debt::LiquidationDebt;
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
        None,
    )?;

    Ok(account_data.get_position())
}

pub fn calc_account_data(
    env: &Env,
    who: &Address,
    cache: &CalcAccountDataCache,
    user_config: &UserConfiguration,
    price_provider: &mut PriceProvider,
    liquidate_debt: Option<Address>,
) -> Result<AccountData, Error> {
    let liquidation = liquidate_debt.is_some();
    if user_config.is_empty() {
        return Ok(AccountData::default());
    }

    let CalcAccountDataCache {
        mb_who_collat,
        mb_who_debt,
        mb_s_token_supply,
        mb_debt_token_supply,
    } = cache;

    let mut total_discounted_collat_in_base: i128 = 0;
    let mut total_debt_in_base: i128 = 0;
    let mut debt_to_cover: Option<_> = None;
    let mut sorted_collat_to_receive = Map::new(env);
    let mut sorted_debt_to_cover = Map::new(env);
    let reserves = read_reserves(env);
    let initial_health = read_initial_health(env)?;
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

            total_discounted_collat_in_base = total_discounted_collat_in_base
                .checked_add(discounted_balance_in_base)
                .ok_or(Error::CalcAccountDataMathError)?;

            if liquidation {
                sorted_collat_to_receive.set(
                    reserve.configuration.liq_order,
                    LiquidationCollateral {
                        reserve_data: reserve,
                        asset,
                        s_token_balance: who_collat,
                        compounded_collat: compounded_balance,
                        collat_coeff: collat_coeff.into_inner(),
                        collat_discount: reserve.configuration.discount,
                    },
                );
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
                let s_token_supply = mb_s_token_supply
                    .filter(|x| x.asset == reserve.s_token_address)
                    .map(|x| x.balance)
                    .unwrap_or_else(|| read_token_total_supply(env, &reserve.s_token_address));

                let debt_token_supply = mb_debt_token_supply
                    .filter(|x| x.asset == reserve.debt_token_address)
                    .map(|x| x.balance)
                    .unwrap_or_else(|| read_token_total_supply(env, &reserve.debt_token_address));

                let utilization =
                    calc_utilization(s_token_supply, debt_token_supply).unwrap_or_default();

                // TODO: what if 2 debts has same utilization?
                // we will skip covering one of it
                sorted_debt_to_cover.set(
                    utilization.into_inner(),
                    LiquidationDebt {
                        asset,
                        reserve_data: reserve,
                        compounded_debt: compounded_balance,
                        debt_token_balance: who_debt,
                        debt_coeff: debt_coeff.into_inner(),
                    },
                );
            }
        }
    }

    let npv = total_discounted_collat_in_base
        .checked_sub(total_debt_in_base)
        .ok_or(Error::CalcAccountDataMathError)?;

    let npv_bp = FixedI128::from_rational(total_discounted_collat_in_base, npv)
        .ok_or(Error::CalcAccountDataMathError)?
        .into_inner()
        .min(PERCENTAGE_FACTOR.into());

    let liq_bonus = npv_bp.min(0).abs().min(PERCENTAGE_FACTOR.into());

    if npv.lt(&0) {
        //
    }

    let initial_health =
        FixedI128::from_percentage(initial_health).ok_or(Error::CalcAccountDataMathError)?;

    let liquidation_data = || -> LiquidationData {
        let sorted = sorted_collat_to_receive.values();
        let mut collat_to_receive = sorted.first().and_then(|v| v.first());
        let is_last_collat = sorted.iter().fold(0, |acc, v| acc + v.len()) == 1;
        if let Some(c) = collat_to_receive.as_mut() {
            c.is_last_collat = is_last_collat;
        }
        LiquidationData {
            debt_to_cover_in_base,
            debt_to_cover,
            collat_to_receive,
        }
    };

    Ok(AccountData {
        discounted_collateral: total_discounted_collat_in_base,
        debt: total_debt_in_base,
        liquidation: liquidation.then_some(liquidation_data()),
        npv,
    })
}
