use common::FixedI128;
use debt_token_interface::DebtTokenClient;
use pool_interface::types::error::Error;
use s_token_interface::STokenClient;
use soroban_sdk::{assert_with_error, token, Address, Env};

use crate::event;
use crate::methods::account_position::account_position;
use crate::storage::{
    add_stoken_underlying_balance, read_token_balance, read_token_total_supply,
    write_token_balance, write_token_total_supply,
};
use crate::types::calc_account_data_cache::CalcAccountDataCache;
use crate::types::liquidation_collateral::LiquidationCollateral;
use crate::types::liquidation_data::LiquidationData;
use crate::types::liquidation_debt::LiquidationDebt;
use crate::types::price_provider::PriceProvider;
use crate::types::user_configurator::UserConfigurator;

use super::account_position::calc_account_data;
use super::utils::recalculate_reserve_data::recalculate_reserve_data;
use super::utils::validation::require_not_paused;

pub fn liquidate(
    env: &Env,
    liquidator: &Address,
    who: &Address,
    asset: Address,
    receive_stoken: bool,
) -> Result<(), Error> {
    liquidator.require_auth();

    require_not_paused(env);

    let mut user_configurator = UserConfigurator::new(env, who, false);
    let user_config = user_configurator.user_config()?;
    let account_data = calc_account_data(
        env,
        who,
        &CalcAccountDataCache::none(),
        user_config,
        &mut PriceProvider::new(env)?,
        Some(asset),
    )?;

    assert_with_error!(env, !account_data.is_good_position(), Error::GoodPosition);

    let liquidation = account_data.liquidation.ok_or(Error::LiquidateMathError)?;

    assert_with_error!(
        env,
        liquidation.debt_to_cover.is_some(),
        Error::MustHaveDebt
    );

    let (covered_debt, liquidated_collateral) = do_liquidate(
        env,
        liquidator,
        who,
        &mut user_configurator,
        &liquidation,
        receive_stoken,
    )?;

    event::liquidation(env, who, covered_debt, liquidated_collateral);

    Ok(())
}

fn do_liquidate(
    env: &Env,
    liquidator: &Address,
    who: &Address,
    user_configurator: &mut UserConfigurator,
    liquidation_data: &LiquidationData,
    receive_stoken: bool,
) -> Result<(i128, i128), Error> {
    let mut remaining_debt_in_base = liquidation_data.debt_to_cover_in_base;
    let mut price_provider = PriceProvider::new(env)?;

    let LiquidationCollateral {
        asset,
        reserve_data: reserve,
        s_token_balance,
        collat_coeff: coll_coeff_fixed,
    } = liquidation_data.collateral_to_receive.first().unwrap();
    let coll_coeff = FixedI128::from_inner(coll_coeff_fixed);
    let compounded_balance = coll_coeff
        .mul_int(s_token_balance)
        .ok_or(Error::LiquidateMathError)?;

    let compounded_balance_in_base = price_provider.convert_to_base(&asset, compounded_balance)?;

    let liq_bonus = FixedI128::from_percentage(reserve.configuration.liq_bonus)
        .ok_or(Error::LiquidateMathError)?;

    let debt_to_cover_in_base_with_penalty = liq_bonus
        .mul_int(remaining_debt_in_base)
        .ok_or(Error::LiquidateMathError)?;

    let (debt_to_cover_in_base, withdraw_amount_in_base) =
        if debt_to_cover_in_base_with_penalty > compounded_balance_in_base {
            // take all available collateral and decrease covered debt by bonus
            let debt_to_cover_in_base =
                (FixedI128::from_inner(2 * FixedI128::DENOMINATOR - liq_bonus.into_inner()))
                    .mul_int(compounded_balance_in_base)
                    .ok_or(Error::LiquidateMathError)?;
            let withdraw_amount_in_base = compounded_balance_in_base;
            (debt_to_cover_in_base, withdraw_amount_in_base)
        } else {
            // take collateral with bonus and cover all debt
            (remaining_debt_in_base, debt_to_cover_in_base_with_penalty)
        };

    let (s_token_amount, underlying_amount) = if withdraw_amount_in_base != compounded_balance_in_base
    {
        let underlying_amount = price_provider.convert_from_base(&asset, withdraw_amount_in_base)?;
        let s_token_amount = coll_coeff
            .recip_mul_int(underlying_amount)
            .ok_or(Error::LiquidateMathError)?;
        (s_token_amount, underlying_amount)
    } else {
        (s_token_balance, compounded_balance)
    };

    let mut who_collat_after = s_token_balance;
    let s_token = STokenClient::new(env, &reserve.s_token_address);
    let mut s_token_supply = read_token_total_supply(env, &reserve.s_token_address);
    let debt_token_supply = read_token_total_supply(env, &reserve.debt_token_address);

    if receive_stoken {
        let mut liquidator_configurator = UserConfigurator::new(env, liquidator, true);
        let liquidator_config = liquidator_configurator.user_config()?;

        assert_with_error!(
            env,
            !liquidator_config.is_borrowing(env, reserve.get_id()),
            Error::MustNotHaveDebt
        );

        let liquidator_collat_before = read_token_balance(env, &s_token.address, liquidator);

        let liquidator_collat_part = s_token_amount;

        if liquidator_collat_part > 0 {
            let liquidator_collat_after = liquidator_collat_before
                .checked_add(liquidator_collat_part)
                .ok_or(Error::MathOverflowError)?;
            who_collat_after = who_collat_after
                .checked_sub(liquidator_collat_part)
                .ok_or(Error::MathOverflowError)?;

            s_token.transfer_on_liquidation(who, liquidator, &liquidator_collat_part);
            write_token_balance(env, &s_token.address, liquidator, liquidator_collat_after)?;
        }

        let use_as_collat = liquidator_collat_before == 0 && liquidator_collat_part > 0;
        let reserve_id = reserve.get_id();

        liquidator_configurator
            .deposit(reserve_id, &asset, use_as_collat)?
            .write();
    } else {
        let amount_to_sub = underlying_amount
            .checked_neg()
            .ok_or(Error::MathOverflowError)?;
        who_collat_after = who_collat_after
            .checked_sub(s_token_amount)
            .ok_or(Error::MathOverflowError)?;
        s_token_supply = s_token_supply
            .checked_sub(s_token_amount)
            .ok_or(Error::MathOverflowError)?;

        s_token.burn(who, &s_token_amount, &underlying_amount, liquidator);
        add_stoken_underlying_balance(env, &s_token.address, amount_to_sub)?;
    }

    // no overflow as withdraw_amount_in_base guaranteed less or equal to to_cover_in_base
    remaining_debt_in_base -= debt_to_cover_in_base;

    let is_withdraw = s_token_balance == s_token_amount;
    user_configurator.withdraw(reserve.get_id(), &asset, is_withdraw)?;

    write_token_total_supply(env, &reserve.s_token_address, s_token_supply)?;
    write_token_balance(env, &s_token.address, who, who_collat_after)?;

    recalculate_reserve_data(env, &asset, &reserve, s_token_supply, debt_token_supply)?;

    let is_last_collateral = liquidation_data.collateral_to_receive.len() == 1;
    assert_with_error!(
        env,
        !is_last_collateral || remaining_debt_in_base == 0,
        Error::NotEnoughCollateral
    );

    for LiquidationDebt {
        asset,
        reserve_data,
        debt_token_balance,
        debt_coeff,
        compounded_debt,
    } in liquidation_data.debt_to_cover.iter()
    {
        let fully_repayed = remaining_debt_in_base == 0;
        let (debt_amount_to_burn, underlying_amount_to_transfer) = if fully_repayed {
            (*debt_token_balance, *compounded_debt)
        } else {
            // no overflow as remaining_debt_with_penalty always less then total_debt_with_penalty_in_base
            let compounded_debt_to_cover = price_provider.convert_from_base(asset, debt_to_cover_in_base)?;
            let debt_to_burn = FixedI128::from_inner(*debt_coeff)
                .recip_mul_int(compounded_debt_to_cover)
                .ok_or(Error::LiquidateMathError)?;
            (debt_to_burn, compounded_debt_to_cover)
        };

        let underlying_asset = token::Client::new(env, asset);
        let debt_token = DebtTokenClient::new(env, &reserve_data.debt_token_address);

        underlying_asset.transfer(
            liquidator,
            &reserve_data.s_token_address,
            &underlying_amount_to_transfer,
        );
        debt_token.burn(who, &debt_amount_to_burn);
        user_configurator.repay(reserve_data.get_id(), fully_repayed)?;

        let mut debt_token_supply = read_token_total_supply(env, &reserve_data.debt_token_address);
        let s_token_supply = read_token_total_supply(env, &reserve_data.s_token_address);

        debt_token_supply = debt_token_supply
            .checked_sub(debt_amount_to_burn)
            .ok_or(Error::MathOverflowError)?;

        add_stoken_underlying_balance(
            env,
            &reserve_data.s_token_address,
            underlying_amount_to_transfer,
        )?;
        write_token_total_supply(env, &reserve_data.debt_token_address, debt_token_supply)?;
        write_token_balance(
            env,
            &debt_token.address,
            who,
            debt_token_balance - debt_amount_to_burn,
        )?;

        recalculate_reserve_data(env, asset, reserve_data, s_token_supply, debt_token_supply)?;
    }

    user_configurator.write();

    Ok((debt_to_cover_in_base, withdraw_amount_in_base))
}
