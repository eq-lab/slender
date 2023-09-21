use common::FixedI128;
use debt_token_interface::DebtTokenClient;
use pool_interface::types::error::Error;
use s_token_interface::STokenClient;
use soroban_sdk::{assert_with_error, token, Address, Env};

use crate::event;
use crate::storage::{
    add_stoken_underlying_balance, read_token_balance, read_token_total_supply,
    write_token_balance, write_token_total_supply,
};
use crate::types::calc_account_data_cache::CalcAccountDataCache;
use crate::types::liquidation_collateral::LiquidationCollateral;
use crate::types::liquidation_data::LiquidationData;
use crate::types::user_configurator::UserConfigurator;

use super::account_position::calc_account_data;
use super::repay::do_repay;
use super::utils::rate::get_actual_borrower_accrued_rate;
use super::utils::recalculate_reserve_data::recalculate_reserve_data;
use super::utils::validation::require_not_paused;

pub fn liquidate(
    env: &Env,
    liquidator: &Address,
    who: &Address,
    receive_stoken: bool,
) -> Result<(), Error> {
    liquidator.require_auth();

    require_not_paused(env);

    let mut user_configurator = UserConfigurator::new(env, who, false);
    let user_config = user_configurator.user_config()?;
    let account_data =
        calc_account_data(env, who, &CalcAccountDataCache::none(), user_config, true)?;

    assert_with_error!(env, !account_data.is_good_position(), Error::GoodPosition);

    let liquidation = account_data.liquidation.ok_or(Error::LiquidateMathError)?;

    do_liquidate(
        env,
        liquidator,
        who,
        &mut user_configurator,
        &liquidation,
        receive_stoken,
    )?;

    event::liquidation(
        env,
        who,
        account_data.debt,
        liquidation.total_debt_with_penalty_in_xlm,
    );

    Ok(())
}

fn do_liquidate(
    env: &Env,
    liquidator: &Address,
    who: &Address,
    user_configurator: &mut UserConfigurator,
    liquidation_data: &LiquidationData,
    receive_stoken: bool,
) -> Result<(), Error> {
    let mut debt_with_penalty = liquidation_data.total_debt_with_penalty_in_xlm;

    for LiquidationCollateral {
        asset,
        reserve_data: reserve,
        s_token_balance,
        asset_price: price_fixed,
        collat_coeff: coll_coeff_fixed,
    } in liquidation_data.collateral_to_receive.iter()
    {
        if debt_with_penalty == 0 {
            break;
        }

        let price = FixedI128::from_inner(price_fixed);

        let coll_coeff = FixedI128::from_inner(coll_coeff_fixed);
        let compounded_balance = coll_coeff
            .mul_int(s_token_balance)
            .ok_or(Error::LiquidateMathError)?;
        let compounded_balance_in_xlm = price
            .mul_int(compounded_balance)
            .ok_or(Error::CalcAccountDataMathError)?;

        let withdraw_amount_in_xlm = compounded_balance_in_xlm.min(debt_with_penalty);
        // no overflow as withdraw_amount_in_xlm guaranteed less or equal to debt_to_cover
        debt_with_penalty -= withdraw_amount_in_xlm;

        let (s_token_amount, underlying_amount) =
            if withdraw_amount_in_xlm != compounded_balance_in_xlm {
                let underlying_amount = price
                    .recip_mul_int(withdraw_amount_in_xlm)
                    .ok_or(Error::LiquidateMathError)?;
                let s_token_amount = coll_coeff
                    .recip_mul_int(underlying_amount)
                    .ok_or(Error::LiquidateMathError)?;
                (s_token_amount, underlying_amount)
            } else {
                (s_token_balance, compounded_balance)
            };

        let s_token = STokenClient::new(env, &reserve.s_token_address);
        let mut s_token_supply = read_token_total_supply(env, &reserve.s_token_address);
        let mut debt_token_supply = read_token_total_supply(env, &reserve.debt_token_address);

        if receive_stoken {
            let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
            let liquidator_debt = read_token_balance(env, &reserve.debt_token_address, liquidator);
            let liquidator_collat_before =
                read_token_balance(env, &reserve.s_token_address, liquidator);

            let mut liquidator_collat_amount = s_token_amount;
            let mut is_debt_repayed = false;

            if liquidator_debt > 0 {
                let debt_coeff = get_actual_borrower_accrued_rate(env, &reserve)?;

                let liquidator_actual_debt = debt_coeff
                    .mul_int(liquidator_debt)
                    .ok_or(Error::LiquidateMathError)?;

                let repayment_amount = liquidator_actual_debt.min(underlying_amount);

                let s_token_to_burn = coll_coeff
                    .recip_mul_int(repayment_amount)
                    .ok_or(Error::LiquidateMathError)?;

                let amount_to_sub = repayment_amount
                    .checked_neg()
                    .ok_or(Error::MathOverflowError)?;

                s_token.burn(who, &s_token_to_burn, &repayment_amount, liquidator);
                add_stoken_underlying_balance(env, &s_token.address, amount_to_sub)?;

                let (is_repayed, debt_token_supply_after) = do_repay(
                    env,
                    liquidator,
                    &asset,
                    &reserve,
                    &debt_token,
                    coll_coeff,
                    debt_coeff,
                    debt_token_supply,
                    repayment_amount,
                )?;
                is_debt_repayed = is_repayed;
                debt_token_supply = debt_token_supply_after;

                liquidator_collat_amount = s_token_amount
                    .checked_sub(s_token_to_burn)
                    .ok_or(Error::LiquidateMathError)?;

                s_token_supply = s_token_supply
                    .checked_sub(s_token_to_burn)
                    .ok_or(Error::MathOverflowError)?;
            }

            if liquidator_collat_amount > 0 {
                s_token.transfer_on_liquidation(who, liquidator, &liquidator_collat_amount);
            }

            let use_as_collat = liquidator_collat_before == 0 && liquidator_collat_amount > 0;
            let reserve_id = reserve.get_id();

            UserConfigurator::new(env, liquidator, true)
                .deposit(reserve_id, &asset, use_as_collat)?
                .repay(reserve_id, is_debt_repayed)?
                .write();
        } else {
            let amount_to_sub = underlying_amount
                .checked_neg()
                .ok_or(Error::MathOverflowError)?;
            let who_collat_after = s_token_balance
                .checked_sub(s_token_amount)
                .ok_or(Error::MathOverflowError)?;
            s_token_supply = s_token_supply
                .checked_sub(s_token_amount)
                .ok_or(Error::MathOverflowError)?;

            s_token.burn(who, &s_token_amount, &underlying_amount, liquidator);

            add_stoken_underlying_balance(env, &s_token.address, amount_to_sub)?;
            write_token_balance(env, &reserve.s_token_address, who, who_collat_after)?;
        }

        let is_withdraw = s_token_balance == s_token_amount;
        user_configurator.withdraw(reserve.get_id(), &asset, is_withdraw)?;

        write_token_total_supply(env, &reserve.s_token_address, s_token_supply)?;
        recalculate_reserve_data(env, &asset, &reserve, s_token_supply, debt_token_supply)?;
    }

    assert_with_error!(env, debt_with_penalty == 0, Error::NotEnoughCollateral);

    for (asset, reserve, compounded_debt, debt_amount) in liquidation_data.debt_to_cover.iter() {
        let underlying_asset = token::Client::new(env, &asset);
        let mut debt_token_supply = read_token_total_supply(env, &reserve.debt_token_address);

        underlying_asset.transfer(liquidator, &reserve.s_token_address, &compounded_debt);
        DebtTokenClient::new(env, &reserve.debt_token_address).burn(who, &debt_amount);
        user_configurator.repay(reserve.get_id(), true)?;

        debt_token_supply = debt_token_supply
            .checked_sub(debt_amount)
            .ok_or(Error::MathOverflowError)?;

        add_stoken_underlying_balance(env, &reserve.s_token_address, compounded_debt)?;
        write_token_total_supply(env, &reserve.debt_token_address, debt_token_supply)?;

        recalculate_reserve_data(
            env,
            &asset,
            &reserve,
            read_token_total_supply(env, &reserve.s_token_address),
            debt_token_supply,
        )?;
    }

    user_configurator.write();

    Ok(())
}
