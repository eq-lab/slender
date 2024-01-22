use common::{FixedI128, PERCENTAGE_FACTOR};
use debt_token_interface::DebtTokenClient;
use pool_interface::types::error::Error;
use s_token_interface::STokenClient;
use soroban_sdk::{assert_with_error, token, Address, Env};

use crate::methods::utils::recalculate_reserve_data::recalculate_reserve_data;
use crate::types::account_data::AccountData;
use crate::types::calc_account_data_cache::CalcAccountDataCache;
use crate::types::price_provider::PriceProvider;
use crate::types::user_configurator::UserConfigurator;
use crate::{
    add_stoken_underlying_balance, event, read_initial_health, read_token_balance,
    read_token_total_supply, write_token_balance, write_token_total_supply,
};

use super::account_position::calc_account_data;
use super::utils::validation::require_not_paused;

pub fn liquidate(
    env: &Env,
    liquidator: &Address,
    who: &Address,
    receive_stoken: bool,
) -> Result<(), Error> {
    // TODO: add user_configurator changes
    // TODO: and liquidator_configurator changes
    // TODO: go through the errors and set the valid ones

    liquidator.require_auth();

    require_not_paused(env);

    let mut user_configurator = UserConfigurator::new(env, who, false);
    let user_config = user_configurator.user_config()?;
    let mut price_provider = PriceProvider::new(env)?;

    let account_data = calc_account_data(
        env,
        who,
        &CalcAccountDataCache::none(),
        user_config,
        &mut price_provider,
        true,
    )?;

    assert_with_error!(env, !account_data.is_good_position(), Error::GoodPosition);

    let (debt_covered_in_base, total_liq_in_base) = do_liquidate(
        env,
        liquidator,
        who,
        account_data,
        &mut user_configurator,
        receive_stoken,
        &mut price_provider,
    )?;

    event::liquidation(env, who, debt_covered_in_base, total_liq_in_base);

    Ok(())
}

fn do_liquidate(
    env: &Env,
    liquidator: &Address,
    who: &Address,
    account_data: AccountData,
    user_configurator: &mut UserConfigurator,
    receive_stoken: bool,
    price_provider: &mut PriceProvider,
) -> Result<(i128, i128), Error> {
    let mut total_debt_after_in_base = account_data.debt;
    let mut total_collat_disc_after_in_base = account_data.discounted_collateral;
    let mut total_debt_to_cover_in_base = 0i128;
    let mut total_liq_in_base = 0i128;

    let initial_health = read_initial_health(env)?;
    let zero_percent = FixedI128::from_inner(0);
    let initial_health =
        FixedI128::from_percentage(initial_health).ok_or(Error::CalcAccountDataMathError)?;
    let hundred_percent =
        FixedI128::from_percentage(PERCENTAGE_FACTOR).ok_or(Error::CalcAccountDataMathError)?;
    let npv_percent = FixedI128::from_rational(account_data.npv, total_collat_disc_after_in_base)
        .ok_or(Error::CalcAccountDataMathError)?;

    let liq_bonus = npv_percent.min(zero_percent).abs().min(hundred_percent);

    let total_debt_liq_bonus = hundred_percent
        .checked_sub(liq_bonus)
        .ok_or(Error::CalcAccountDataMathError)?;

    for collat in account_data
        .liq_collats
        .ok_or(Error::CalcAccountDataMathError)?
    {
        let discount = FixedI128::from_percentage(collat.reserve.configuration.discount)
            .ok_or(Error::CalcAccountDataMathError)?;

        let safe_collat_in_base = hundred_percent
            .checked_sub(initial_health)
            .unwrap()
            .mul_int(total_collat_disc_after_in_base)
            .ok_or(Error::CalcAccountDataMathError)?
            .checked_sub(total_debt_after_in_base)
            .ok_or(Error::CalcAccountDataMathError)?;

        let safe_discount_level = discount
            .checked_mul(initial_health)
            .ok_or(Error::CalcAccountDataMathError)?;

        let safe_discount = discount
            .checked_add(liq_bonus)
            .ok_or(Error::CalcAccountDataMathError)?
            .checked_sub(hundred_percent)
            .ok_or(Error::CalcAccountDataMathError)?
            .checked_sub(safe_discount_level)
            .ok_or(Error::CalcAccountDataMathError)?;

        let liq_comp_amount =
            price_provider.convert_from_base(&collat.asset, safe_collat_in_base)?;

        let liq_comp_amount = safe_discount
            .recip_mul_int(liq_comp_amount)
            .ok_or(Error::CalcAccountDataMathError)?;

        let liq_max_comp_amount = liq_comp_amount
            .is_negative()
            .then(|| collat.comp_balance)
            .unwrap_or_else(|| collat.comp_balance.min(liq_comp_amount));

        total_liq_in_base = total_liq_in_base
            .checked_add(price_provider.convert_to_base(&collat.asset, liq_max_comp_amount)?)
            .ok_or(Error::CalcAccountDataMathError)?;

        let total_sub_comp_amount = discount
            .mul_int(liq_max_comp_amount)
            .ok_or(Error::CalcAccountDataMathError)?;

        let total_sub_amount_in_base =
            price_provider.convert_to_base(&collat.asset, total_sub_comp_amount)?;

        let debt_comp_amount = total_debt_liq_bonus
            .mul_int(liq_max_comp_amount)
            .ok_or(Error::CalcAccountDataMathError)?;

        let debt_in_base = price_provider.convert_to_base(&collat.asset, debt_comp_amount)?;

        total_debt_after_in_base = total_debt_after_in_base
            .checked_sub(debt_in_base)
            .ok_or(Error::CalcAccountDataMathError)?;

        total_collat_disc_after_in_base = total_collat_disc_after_in_base
            .checked_sub(total_sub_amount_in_base)
            .ok_or(Error::CalcAccountDataMathError)?;

        let npv_after = total_collat_disc_after_in_base
            .checked_sub(total_debt_after_in_base)
            .ok_or(Error::CalcAccountDataMathError)?;

        let s_token = STokenClient::new(env, &collat.reserve.s_token_address);

        let mut s_token_supply = read_token_total_supply(env, &collat.reserve.s_token_address);
        let debt_token_supply = read_token_total_supply(env, &collat.reserve.debt_token_address);

        let liq_lp_amount = FixedI128::from_inner(collat.coeff)
            .recip_mul_int(liq_max_comp_amount)
            .ok_or(Error::LiquidateMathError)?;

        if receive_stoken {
            let mut liquidator_configurator = UserConfigurator::new(env, liquidator, true);
            let liquidator_config = liquidator_configurator.user_config()?;

            assert_with_error!(
                env,
                !liquidator_config.is_borrowing(env, collat.reserve.get_id()),
                Error::MustNotHaveDebt
            );

            let liquidator_collat_before = read_token_balance(env, &s_token.address, liquidator);

            let liquidator_collat_after = liquidator_collat_before
                .checked_add(liq_lp_amount)
                .ok_or(Error::MathOverflowError)?;

            s_token.transfer_on_liquidation(who, liquidator, &liq_lp_amount);
            write_token_balance(env, &s_token.address, liquidator, liquidator_collat_after)?;

            let use_as_collat = liquidator_collat_before == 0;

            liquidator_configurator
                .deposit(collat.reserve.get_id(), &collat.asset, use_as_collat)?
                .write();
        } else {
            let amount_to_sub = liq_lp_amount
                .checked_neg()
                .ok_or(Error::LiquidateMathError)?;
            s_token_supply = s_token_supply
                .checked_sub(liq_lp_amount)
                .ok_or(Error::MathOverflowError)?;

            s_token.burn(who, &liq_lp_amount, &liq_max_comp_amount, liquidator);
            add_stoken_underlying_balance(env, &s_token.address, amount_to_sub)?;
        }

        write_token_total_supply(env, &collat.reserve.s_token_address, s_token_supply)?;

        recalculate_reserve_data(
            env,
            &collat.asset,
            &collat.reserve,
            s_token_supply,
            debt_token_supply,
        )?;

        total_debt_to_cover_in_base += debt_in_base;

        if npv_after.is_positive() {
            break;
        }
    }

    let debt_covered_in_base = total_debt_to_cover_in_base;

    for debt in account_data
        .liq_debts
        .ok_or(Error::CalcAccountDataMathError)?
    {
        if total_debt_to_cover_in_base.eq(&0) {
            break;
        }

        let debt_comp_in_base = price_provider.convert_to_base(&debt.asset, debt.comp_balance)?;

        let (debt_lp_to_burn, debt_comp_to_transfer) =
            if total_debt_to_cover_in_base >= debt_comp_in_base {
                total_debt_to_cover_in_base -= debt_comp_in_base;

                user_configurator.repay(debt.reserve.get_id(), true)?;

                (debt.lp_balance, debt.comp_balance)
            } else {
                let debt_comp_amount =
                    price_provider.convert_from_base(&debt.asset, total_debt_to_cover_in_base)?;

                let debt_lp_amount = FixedI128::from_inner(debt.coeff)
                    .recip_mul_int(debt_comp_amount)
                    .ok_or(Error::LiquidateMathError)?;

                total_debt_to_cover_in_base = 0;

                (debt_lp_amount, debt_comp_amount)
            };

        let underlying_asset = token::Client::new(env, &debt.asset);
        let debt_token = DebtTokenClient::new(env, &debt.reserve.debt_token_address);

        underlying_asset.transfer(
            liquidator,
            &debt.reserve.s_token_address,
            &debt_comp_to_transfer,
        );

        debt_token.burn(who, &debt_lp_to_burn);

        let mut debt_token_supply = read_token_total_supply(env, &debt.reserve.debt_token_address);
        let s_token_supply = read_token_total_supply(env, &debt.reserve.s_token_address);

        debt_token_supply = debt_token_supply
            .checked_sub(debt_lp_to_burn)
            .ok_or(Error::MathOverflowError)?;

        add_stoken_underlying_balance(env, &debt.reserve.s_token_address, debt_comp_to_transfer)?;
        write_token_total_supply(env, &debt.reserve.debt_token_address, debt_token_supply)?;
        write_token_balance(
            env,
            &debt_token.address,
            who,
            debt.lp_balance - debt_lp_to_burn,
        )?;

        recalculate_reserve_data(
            env,
            &debt.asset,
            &debt.reserve,
            s_token_supply,
            debt_token_supply,
        )?;
    }

    user_configurator.write();

    Ok((debt_covered_in_base, total_liq_in_base))
}
