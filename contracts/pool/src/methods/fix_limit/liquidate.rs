use common::FixedI128;
use pool_interface::types::asset_balance::AssetBalance;
use pool_interface::types::error::Error;
use pool_interface::types::mint_burn::MintBurn;
use soroban_sdk::{assert_with_error, vec, Address, Env, Vec};

use crate::event;
use crate::methods::fix_limit::account_position::{calc_account_data, CalcAccountDataCache};
use crate::methods::fix_limit::repay::do_repay;
use crate::methods::utils::rate::get_actual_borrower_accrued_rate;
use crate::methods::utils::recalculate_reserve_data::recalculate_reserve_data;
use crate::methods::utils::validation::require_not_paused;
use crate::storage::{
    add_stoken_underlying_balance, add_token_balance, add_token_total_supply, read_token_balance,
    read_token_total_supply,
};
use crate::types::liquidation_collateral::LiquidationCollateral;
use crate::types::liquidation_data::LiquidationData;
use crate::types::user_configurator::UserConfigurator;

pub fn liquidate(
    env: &Env,
    liquidator: &Address,
    who: &Address,
    receive_stoken: bool,
) -> Result<Vec<MintBurn>, Error> {
    liquidator.require_auth();

    require_not_paused(env);

    let mut user_configurator = UserConfigurator::new(env, who, false);
    let user_config = user_configurator.user_config()?;
    let account_data =
        calc_account_data(env, who, CalcAccountDataCache::none(), user_config, true)?;

    assert_with_error!(env, !account_data.is_good_position(), Error::GoodPosition);

    let liquidation = account_data.liquidation.ok_or(Error::LiquidateMathError)?;

    let mint_burn_vec = do_liquidate(
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

    Ok(mint_burn_vec)
}

fn do_liquidate(
    env: &Env,
    liquidator: &Address,
    who: &Address,
    user_configurator: &mut UserConfigurator,
    liquidation_data: &LiquidationData,
    receive_stoken: bool,
) -> Result<Vec<MintBurn>, Error> {
    let mut debt_with_penalty = liquidation_data.total_debt_with_penalty_in_xlm;

    let mut mint_burn_vec = vec![env];

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

        let mut s_token_supply = read_token_total_supply(env, &reserve.s_token_address);
        let mut debt_token_supply = read_token_total_supply(env, &reserve.debt_token_address);

        if receive_stoken {
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

                mint_burn_vec.push_back(MintBurn::new(
                    AssetBalance::new(reserve.s_token_address.clone(), s_token_to_burn),
                    false,
                    who.clone(),
                ));
                add_token_total_supply(
                    env,
                    &reserve.s_token_address,
                    s_token_to_burn.checked_neg().unwrap(),
                )?;
                mint_burn_vec.push_back(MintBurn::new(
                    AssetBalance::new(asset.clone(), repayment_amount),
                    true,
                    liquidator.clone(),
                ));
                add_token_balance(
                    env,
                    &reserve.s_token_address,
                    who,
                    s_token_to_burn.checked_neg().unwrap(),
                )?;
                add_stoken_underlying_balance(env, &reserve.s_token_address, amount_to_sub)?;

                let (is_repayed, debt_token_supply_after, mint_burn_repay) = do_repay(
                    env,
                    liquidator,
                    &asset,
                    &reserve,
                    coll_coeff,
                    debt_coeff,
                    debt_token_supply,
                    liquidator_debt,
                    repayment_amount,
                )?;

                for repay in mint_burn_repay {
                    mint_burn_vec.push_back(repay);
                }

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
                mint_burn_vec.push_back(MintBurn::new(
                    AssetBalance::new(reserve.s_token_address.clone(), liquidator_collat_amount),
                    false,
                    who.clone(),
                ));
                mint_burn_vec.push_back(MintBurn::new(
                    AssetBalance::new(reserve.s_token_address.clone(), liquidator_collat_amount),
                    true,
                    liquidator.clone(),
                ));

                add_token_balance(
                    env,
                    &reserve.s_token_address,
                    who,
                    liquidator_collat_amount.checked_neg().unwrap(),
                )?;
                add_token_balance(
                    env,
                    &reserve.s_token_address,
                    liquidator,
                    liquidator_collat_amount,
                )?;
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
            s_token_supply = s_token_supply
                .checked_sub(s_token_amount)
                .ok_or(Error::MathOverflowError)?;

            mint_burn_vec.push_back(MintBurn::new(
                AssetBalance::new(reserve.s_token_address.clone(), s_token_amount),
                false,
                who.clone(),
            ));
            add_token_total_supply(
                env,
                &reserve.s_token_address,
                s_token_amount.checked_neg().unwrap(),
            )?;
            mint_burn_vec.push_back(MintBurn::new(
                AssetBalance::new(asset.clone(), underlying_amount),
                true,
                liquidator.clone(),
            ));
            add_token_balance(
                env,
                &reserve.s_token_address,
                who,
                s_token_amount.checked_neg().unwrap(),
            )?;

            add_stoken_underlying_balance(env, &reserve.s_token_address, amount_to_sub)?;
        }

        let is_withdraw = s_token_balance == s_token_amount;
        user_configurator.withdraw(reserve.get_id(), &asset, is_withdraw)?;

        recalculate_reserve_data(env, &asset, &reserve, s_token_supply, debt_token_supply)?;
    }

    assert_with_error!(env, debt_with_penalty == 0, Error::NotEnoughCollateral);

    for (asset, reserve, compounded_debt, debt_amount) in liquidation_data.debt_to_cover.iter() {
        // let s_token = STokenClient::new(env, &reserve.s_token_address);
        let s_token_supply = read_token_total_supply(env, &reserve.s_token_address);
        // let underlying_asset = token::Client::new(env, &s_token.underlying_asset());
        // let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);

        mint_burn_vec.push_back(MintBurn::new(
            AssetBalance::new(asset.clone(), compounded_debt),
            false,
            liquidator.clone(),
        ));
        mint_burn_vec.push_back(MintBurn::new(
            AssetBalance::new(asset.clone(), compounded_debt),
            true,
            reserve.s_token_address.clone(),
        ));
        add_stoken_underlying_balance(env, &reserve.s_token_address, compounded_debt)?;

        mint_burn_vec.push_back(MintBurn::new(
            AssetBalance::new(reserve.debt_token_address.clone(), debt_amount),
            false,
            who.clone(),
        ));
        add_token_balance(
            env,
            &reserve.debt_token_address,
            who,
            debt_amount.checked_neg().unwrap(),
        )?;
        add_token_total_supply(
            env,
            &reserve.debt_token_address,
            debt_amount.checked_neg().unwrap(),
        )?;
        user_configurator.repay(reserve.get_id(), true)?;

        recalculate_reserve_data(
            env,
            &asset,
            &reserve,
            s_token_supply,
            read_token_total_supply(env, &reserve.debt_token_address),
        )?;
    }

    user_configurator.write();

    Ok(mint_burn_vec)
}
