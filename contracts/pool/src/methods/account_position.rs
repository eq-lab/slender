use common::FixedI128;
use pool_interface::types::account_position::AccountPosition;
use pool_interface::types::error::Error;
use pool_interface::types::reserve_data::ReserveData;
use pool_interface::types::reserve_type::ReserveType;
use pool_interface::types::user_config::UserConfiguration;
use soroban_sdk::{assert_with_error, Address, Env, Map, Vec};

use crate::storage::{
    read_reserve, read_reserves, read_token_balance, read_token_total_supply, read_user_config,
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

    let mut total_discounted_collateral_in_base: i128 = 0;
    let mut total_debt_in_base: i128 = 0;
    let mut debt_to_cover_in_base: i128 = 0;
    let mut debt_to_cover: Option<_> = None;
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

        if let ReserveType::Fungible(s_token_address, debt_token_address) = reserve.reserve_type {
            calculate_fungible(
                env,
                who,
                user_config,
                cache,
                reserve,
                s_token_address,
                debt_token_address,
                asset,
                liquidate_debt,
                price_provider,
                &mut sorted_collateral_to_receive,
                &mut total_discounted_collateral_in_base,
                &mut total_debt_in_base,
                &mut debt_to_cover_in_base,
                &mut debt_to_cover
            )?;
        } else {

        }
    }

    let npv = total_discounted_collateral_in_base
        .checked_sub(total_debt_in_base)
        .ok_or(Error::CalcAccountDataMathError)?;

    let liquidation_data = || -> LiquidationData {
        let sorted = sorted_collateral_to_receive.values();
        let mut collateral_to_receive = sorted.first().and_then(|v| v.first());
        let is_last_collateral = sorted.iter().fold(0, |acc, v| acc + v.len()) == 1;
        if let Some(c) = collateral_to_receive.as_mut() {
            c.is_last_collateral = is_last_collateral;
        }
        LiquidationData {
            debt_to_cover_in_base,
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

fn calculate_fungible(
    env: &Env,
    who: &Address,
    user_config: &UserConfiguration,
    cache: &CalcAccountDataCache,
    reserve: ReserveData,
    s_token_address: Address,
    debt_token_address: Address,
    asset: Address,
    liquidate_debt: Option<Address>,
    price_provider: &mut PriceProvider,
    sorted_collateral_to_receive: &mut Map<u32, Vec<LiquidationCollateral>>,
    total_discounted_collateral_in_base: &mut i128,
    total_debt_in_base: &mut i128,
    debt_to_cover_in_base: &mut i128,
    debt_to_cover: &mut Option<LiquidationDebt>
) -> Result<(), Error> {
    let liquidation = liquidate_debt.is_some();
    let CalcAccountDataCache {
        mb_who_collat,
        mb_who_debt,
        mb_s_token_supply,
        mb_debt_token_supply,
    } = cache;

    let reserve_index = reserve.get_id();
    if user_config.is_using_as_collateral(env, reserve_index) {
        let s_token_supply = mb_s_token_supply
            .filter(|x| x.asset == s_token_address)
            .map(|x| x.balance)
            .unwrap_or_else(|| read_token_total_supply(env, &s_token_address));
        let debt_token_supply = mb_debt_token_supply
            .filter(|x| x.asset == debt_token_address)
            .map(|x| x.balance)
            .unwrap_or_else(|| read_token_total_supply(env, &debt_token_address));

        let collat_coeff = get_collat_coeff(env, &reserve, &s_token_address, s_token_supply, debt_token_supply)?;

        let s_token_balance = mb_who_collat
            .filter(|x| x.asset == s_token_address)
            .map(|x| x.balance)
            .unwrap_or_else(|| read_token_balance(env, &s_token_address, who));

        let discount = FixedI128::from_percentage(reserve.configuration.discount)
            .ok_or(Error::CalcAccountDataMathError)?;

        let compounded_balance = collat_coeff
            .mul_int(s_token_balance)
            .ok_or(Error::CalcAccountDataMathError)?;

        let compounded_balance_in_base =
            price_provider.convert_to_base(&asset, compounded_balance)?;

        let discounted_balance_in_base = discount
            .mul_int(compounded_balance_in_base)
            .ok_or(Error::CalcAccountDataMathError)?;

        *total_discounted_collateral_in_base = total_discounted_collateral_in_base
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
                s_token_balance: Some(s_token_balance),
                collat_coeff: Some(collat_coeff.into_inner()),
                compounded_collat: compounded_balance,
                is_last_collateral: Default::default(),
            });
            sorted_collateral_to_receive.set(curr_discount, collateral_to_receive);
        }
    } else if user_config.is_borrowing(env, reserve_index) {
        let debt_coeff = get_actual_borrower_accrued_rate(env, &reserve)?;

        let who_debt = mb_who_debt
            .filter(|x| x.asset == debt_token_address)
            .map(|x| x.balance)
            .unwrap_or_else(|| read_token_balance(env, &debt_token_address, who));

        let compounded_balance = debt_coeff
            .mul_int(who_debt)
            .ok_or(Error::CalcAccountDataMathError)?;

        let debt_balance_in_base =
            price_provider.convert_to_base(&asset, compounded_balance)?;

        *total_debt_in_base = total_debt_in_base
            .checked_add(debt_balance_in_base)
            .ok_or(Error::CalcAccountDataMathError)?;

        if liquidate_debt
            .as_ref()
            .map(|debt| debt == &asset)
            .unwrap_or(false)
        {
            *debt_to_cover_in_base = debt_balance_in_base;

            *debt_to_cover = Some(LiquidationDebt {
                asset,
                reserve_data: reserve,
                compounded_debt: compounded_balance,
                debt_token_balance: who_debt,
                debt_coeff: debt_coeff.into_inner(),
            });
        }
    }

    Ok(())
}

fn calculate_rwa(
    env: &Env,
    who: &Address,
    user_config: &UserConfiguration,
    reserve: ReserveData,
    s_token_address: Address,
    debt_token_address: Address,
    asset: Address,
    liquidate_debt: Option<Address>,
    price_provider: &mut PriceProvider,
    sorted_collateral_to_receive: &mut Map<u32, Vec<LiquidationCollateral>>,
    total_discounted_collateral_in_base: &mut i128,
    total_debt_in_base: &mut i128,
    debt_to_cover_in_base: &mut i128,
    debt_to_cover: &mut Option<LiquidationDebt>
) -> Result<(), Error> {
    let liquidation = liquidate_debt.is_some();
    let reserve_index = reserve.get_id();
    if user_config.is_using_as_collateral(env, reserve_index) {
        let discount = FixedI128::from_percentage(reserve.configuration.discount)
            .ok_or(Error::CalcAccountDataMathError)?;

        // TODO: depence on RWA implementation
        let balance = 1;

        let balance_in_base =
            price_provider.convert_to_base(&asset, balance)?;

        let discounted_balance_in_base = discount
            .mul_int(balance_in_base)
            .ok_or(Error::CalcAccountDataMathError)?;

        *total_discounted_collateral_in_base = total_discounted_collateral_in_base
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
                s_token_balance: None,
                collat_coeff: None,
                compounded_collat: balance,
                is_last_collateral: Default::default(),
            });
            sorted_collateral_to_receive.set(curr_discount, collateral_to_receive);
        }
    }

    Ok(())
}