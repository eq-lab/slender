use common::FixedI128;
use pool_interface::types::account_position::AccountPosition;
use pool_interface::types::error::Error;
use pool_interface::types::pool_config::PoolConfig;
use pool_interface::types::reserve_data::ReserveData;
use pool_interface::types::reserve_type::ReserveType;
use pool_interface::types::user_config::UserConfiguration;
use soroban_sdk::{assert_with_error, Address, Env, Map, Vec};

use crate::storage::{
    read_reserve, read_reserves, read_token_balance, read_token_total_supply, read_user_config,
};
use crate::types::account_data::AccountData;
use crate::types::calc_account_data_cache::CalcAccountDataCache;
use crate::types::liquidation_asset::LiquidationAsset;
use crate::types::price_provider::PriceProvider;

use super::utils::get_collat_coeff::get_compounded_amount;
use super::utils::rate::get_actual_borrower_accrued_rate;

pub fn account_position(
    env: &Env,
    who: &Address,
    pool_config: &PoolConfig,
) -> Result<AccountPosition, Error> {
    let user_config = read_user_config(env, who)?;
    let account_data = calc_account_data(
        env,
        who,
        &CalcAccountDataCache::none(),
        pool_config,
        &user_config,
        &mut PriceProvider::new(env, pool_config)?,
        false,
    )?;

    Ok(account_data.get_position())
}

pub fn calc_account_data(
    env: &Env,
    who: &Address,
    cache: &CalcAccountDataCache,
    pool_config: &PoolConfig,
    user_config: &UserConfiguration,
    price_provider: &mut PriceProvider,
    liquidation: bool,
) -> Result<AccountData, Error> {
    if user_config.is_empty() {
        return Ok(AccountData::default());
    }

    let mut total_discounted_collat_in_base: i128 = 0;
    let mut total_collat_in_base: i128 = 0;
    let mut total_debt_in_base: i128 = 0;
    let mut sorted_collat_to_receive = Map::new(env);
    let mut sorted_debt_to_cover = Map::new(env);
    let reserves = read_reserves(env);
    let reserves_len =
        u8::try_from(reserves.len()).map_err(|_| Error::ReservesMaxCapacityExceeded)?;

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

        calculate(
            env,
            who,
            user_config,
            cache,
            reserve,
            pool_config,
            asset,
            liquidation,
            price_provider,
            &mut sorted_collat_to_receive,
            &mut total_collat_in_base,
            &mut total_discounted_collat_in_base,
            &mut total_debt_in_base,
            &mut sorted_debt_to_cover,
        )?;
    }

    let npv = total_discounted_collat_in_base
        .checked_sub(total_debt_in_base)
        .ok_or(Error::CalcAccountDataMathError)?;

    let sorted_debt_to_pay = || -> Vec<LiquidationAsset> {
        let mut result = Vec::new(env);

        for debt in sorted_debt_to_cover.values().into_iter().flatten() {
            result.push_front(debt);
        }

        result
    };

    Ok(AccountData {
        discounted_collateral: total_discounted_collat_in_base,
        debt: total_debt_in_base,
        collat: liquidation.then_some(total_collat_in_base),
        liq_debts: liquidation.then_some(sorted_debt_to_pay()),
        liq_collats: liquidation.then_some(sorted_collat_to_receive.values()),
        npv,
    })
}

#[allow(clippy::too_many_arguments)]
fn calculate(
    env: &Env,
    who: &Address,
    user_config: &UserConfiguration,
    cache: &CalcAccountDataCache,
    reserve: ReserveData,
    pool_config: &PoolConfig,
    asset: Address,
    liquidation: bool,
    price_provider: &mut PriceProvider,
    sorted_collat_to_receive: &mut Map<u32, LiquidationAsset>,
    total_collat_in_base: &mut i128,
    total_discounted_collat_in_base: &mut i128,
    total_debt_in_base: &mut i128,
    sorted_debt_to_cover: &mut Map<i128, Vec<LiquidationAsset>>,
) -> Result<(), Error> {
    let CalcAccountDataCache {
        mb_who_collat,
        mb_who_debt,
        mb_s_token_supply,
        mb_debt_token_supply,
        mb_s_token_underlying_balance,
        mb_rwa_balance,
    } = cache;

    let reserve_index = reserve.get_id();
    if user_config.is_using_as_collateral(env, reserve_index) {
        let discount = FixedI128::from_percentage(reserve.configuration.discount)
            .ok_or(Error::CalcAccountDataMathError)?;
        let (balance, who_collat) =
            if let ReserveType::Fungible(s_token_address, debt_token_address) =
                reserve.reserve_type.clone()
            {
                let s_token_supply = mb_s_token_supply
                    .filter(|x| x.asset == s_token_address)
                    .map(|x| x.balance)
                    .unwrap_or_else(|| read_token_total_supply(env, &s_token_address));

                let debt_token_supply = mb_debt_token_supply
                    .filter(|x| x.asset == debt_token_address)
                    .map(|x| x.balance)
                    .unwrap_or_else(|| read_token_total_supply(env, &debt_token_address));

                let s_token_underlying_balance = mb_s_token_underlying_balance
                    .filter(|x| x.asset == s_token_address)
                    .map(|x| x.balance)
                    .unwrap_or_else(|| read_token_balance(env, &asset, &s_token_address));

                let who_collat = mb_who_collat
                    .filter(|x| x.asset == s_token_address)
                    .map(|x| x.balance)
                    .unwrap_or_else(|| read_token_balance(env, &s_token_address, who));

                (
                    get_compounded_amount(
                        env,
                        &reserve,
                        pool_config,
                        s_token_supply,
                        s_token_underlying_balance,
                        debt_token_supply,
                        who_collat,
                    )?,
                    Some(who_collat),
                )
            } else {
                (
                    mb_rwa_balance
                        .filter(|x| x.asset == asset)
                        .map(|x| x.balance)
                        .unwrap_or_else(|| read_token_balance(env, &asset, who)),
                    None,
                )
            };

        let balance_in_base = price_provider.convert_to_base(&asset, balance)?;

        let discounted_balance_in_base = discount
            .mul_int(balance_in_base)
            .ok_or(Error::CalcAccountDataMathError)?;

        *total_discounted_collat_in_base = total_discounted_collat_in_base
            .checked_add(discounted_balance_in_base)
            .ok_or(Error::CalcAccountDataMathError)?;

        if liquidation {
            *total_collat_in_base = total_collat_in_base
                .checked_add(balance_in_base)
                .ok_or(Error::CalcAccountDataMathError)?;

            sorted_collat_to_receive.set(
                reserve.configuration.pen_order,
                LiquidationAsset {
                    asset,
                    reserve,
                    coeff: None,
                    lp_balance: who_collat,
                    comp_balance: balance,
                },
            );
        }
    } else if user_config.is_borrowing(env, reserve_index) {
        if let ReserveType::Fungible(s_token_address, debt_token_address) =
            reserve.reserve_type.clone()
        {
            let debt_coeff = get_actual_borrower_accrued_rate(env, &reserve, pool_config)?;

            let who_debt = mb_who_debt
                .filter(|x| x.asset == debt_token_address)
                .map(|x| x.balance)
                .unwrap_or_else(|| read_token_balance(env, &debt_token_address, who));

            let compounded_debt = debt_coeff
                .mul_int(who_debt)
                .ok_or(Error::CalcAccountDataMathError)?;

            let debt_balance_in_base = price_provider.convert_to_base(&asset, compounded_debt)?;

            *total_debt_in_base = total_debt_in_base
                .checked_add(debt_balance_in_base)
                .ok_or(Error::CalcAccountDataMathError)?;

            if liquidation {
                let s_token_supply = mb_s_token_supply
                    .filter(|x| x.asset == s_token_address)
                    .map(|x| x.balance)
                    .unwrap_or_else(|| read_token_total_supply(env, &s_token_address));

                let debt_token_supply = mb_debt_token_supply
                    .filter(|x| x.asset == debt_token_address)
                    .map(|x| x.balance)
                    .unwrap_or_else(|| read_token_total_supply(env, &debt_token_address));

                let utilization = FixedI128::from_rational(debt_token_supply, s_token_supply)
                    .ok_or(Error::CalcAccountDataMathError)?
                    .into_inner();

                let mut debt_to_cover = sorted_debt_to_cover
                    .get(utilization)
                    .unwrap_or(Vec::new(env));

                debt_to_cover.push_back(LiquidationAsset {
                    asset,
                    reserve,
                    coeff: Some(debt_coeff.into_inner()),
                    lp_balance: Some(who_debt),
                    comp_balance: compounded_debt,
                });

                sorted_debt_to_cover.set(utilization, debt_to_cover);
            }
        }
    }

    Ok(())
}
