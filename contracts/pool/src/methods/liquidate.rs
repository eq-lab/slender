use common::{FixedI128, PERCENTAGE_FACTOR};
use debt_token_interface::DebtTokenClient;
use pool_interface::types::error::Error;
use pool_interface::types::reserve_type::ReserveType;
use s_token_interface::STokenClient;
use soroban_sdk::{assert_with_error, token, Address, Env};

use crate::methods::utils::recalculate_reserve_data::recalculate_reserve_data;
use crate::methods::utils::validation::require_not_in_grace_period;
use crate::types::account_data::AccountData;
use crate::types::calc_account_data_cache::CalcAccountDataCache;
use crate::types::liquidation_asset::LiquidationAsset;
use crate::types::price_provider::PriceProvider;
use crate::types::user_configurator::UserConfigurator;
use crate::{
    add_protocol_fee_vault, add_stoken_underlying_balance, event, read_initial_health,
    read_liquidation_protocol_fee, read_pause_info, read_stoken_underlying_balance,
    read_token_balance, read_token_total_supply, write_token_balance, write_token_total_supply,
};

use super::account_position::calc_account_data;
use super::utils::get_collat_coeff::get_lp_amount;
use super::utils::validation::require_not_paused;

pub fn liquidate(env: &Env, liquidator: &Address, who: &Address) -> Result<(), Error> {
    liquidator.require_auth();

    let pause_info = read_pause_info(env)?;
    require_not_paused(env, &pause_info);
    require_not_in_grace_period(env, &pause_info);

    let mut user_configurator = UserConfigurator::new(env, who, false, None);
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
    price_provider: &mut PriceProvider,
) -> Result<(i128, i128), Error> {
    let mut total_debt_after_in_base = account_data.debt;
    let mut total_collat_disc_after_in_base = account_data.discounted_collateral;
    let mut total_debt_to_cover_in_base = 0i128;
    let mut total_liq_in_base = 0i128;
    let mut debt_covered_in_base = 0i128;
    let total_collat_in_base = account_data.collat.ok_or(Error::LiquidateMathError)?;

    let zero_percent = FixedI128::from_inner(0);
    let initial_health_percent = FixedI128::from_percentage(read_initial_health(env)?).unwrap();
    let hundred_percent = FixedI128::from_percentage(PERCENTAGE_FACTOR).unwrap();
    let npv_percent = FixedI128::from_rational(account_data.npv, total_collat_disc_after_in_base)
        .ok_or(Error::LiquidateMathError)?;

    let liq_bonus_percent = npv_percent.min(zero_percent).abs().min(hundred_percent);
    let max_liq_bonus_percent = FixedI128::from_rational(
        total_collat_in_base
            .checked_sub(account_data.debt)
            .ok_or(Error::LiquidateMathError)?,
        total_collat_in_base,
    )
    .ok_or(Error::LiquidateMathError)?;

    let full_liquidation = max_liq_bonus_percent <= liq_bonus_percent;

    let (total_debt_liq_bonus_percent, safe_collat_percent) = if !full_liquidation {
        let total_debt_liq_bonus_percent = hundred_percent
            .checked_sub(liq_bonus_percent)
            .ok_or(Error::LiquidateMathError)?;

        let safe_collat_percent = hundred_percent.checked_sub(initial_health_percent).unwrap();

        (total_debt_liq_bonus_percent, safe_collat_percent)
    } else {
        (FixedI128::ZERO, FixedI128::ZERO)
    };

    let liquidation_protocol_fee = FixedI128::from_percentage(read_liquidation_protocol_fee(env))
        .ok_or(Error::MathOverflowError)?;

    for collat in account_data.liq_collats.ok_or(Error::LiquidateMathError)? {
        let (liq_comp_amount, debt_in_base) = if !full_liquidation {
            let discount_percent =
                FixedI128::from_percentage(collat.reserve.configuration.discount).unwrap();

            // the same for token-based RWA
            let liq_comp_amount = calc_liq_amount(
                price_provider,
                &collat,
                hundred_percent,
                discount_percent,
                liq_bonus_percent,
                safe_collat_percent,
                initial_health_percent,
                total_collat_disc_after_in_base,
                total_debt_after_in_base,
            )?;

            let total_sub_comp_amount = discount_percent
                .mul_int(liq_comp_amount)
                .ok_or(Error::LiquidateMathError)?;

            let total_sub_amount_in_base =
                price_provider.convert_to_base(&collat.asset, total_sub_comp_amount)?;

            let debt_comp_amount = total_debt_liq_bonus_percent
                .mul_int_ceil(liq_comp_amount)
                .ok_or(Error::LiquidateMathError)?;

            let debt_in_base = price_provider.convert_to_base(&collat.asset, debt_comp_amount)?;

            total_debt_after_in_base = total_debt_after_in_base
                .checked_sub(debt_in_base)
                .ok_or(Error::LiquidateMathError)?;

            total_collat_disc_after_in_base = total_collat_disc_after_in_base
                .checked_sub(total_sub_amount_in_base)
                .ok_or(Error::LiquidateMathError)?;

            (liq_comp_amount, debt_in_base)
        } else {
            (collat.comp_balance, 0)
        };

        total_liq_in_base = total_liq_in_base
            .checked_add(price_provider.convert_to_base(&collat.asset, liq_comp_amount)?)
            .ok_or(Error::LiquidateMathError)?;

        let protocol_part_underlying = liquidation_protocol_fee
            .mul_int(liq_comp_amount)
            .ok_or(Error::MathOverflowError)?;

        let liquidator_part_underlying = liq_comp_amount - protocol_part_underlying;

        if let ReserveType::Fungible(s_token_address, debt_token_address) =
            &collat.reserve.reserve_type
        {
            let mut s_token_supply = read_token_total_supply(env, s_token_address);
            let debt_token_supply = read_token_total_supply(env, debt_token_address);

            let liq_lp_amount = if !full_liquidation && liq_comp_amount < collat.comp_balance {
                get_lp_amount(
                    env,
                    &collat.reserve,
                    s_token_supply,
                    read_stoken_underlying_balance(env, s_token_address),
                    debt_token_supply,
                    liq_comp_amount,
                )?
            } else {
                collat.lp_balance.unwrap()
            };

            let s_token = STokenClient::new(env, s_token_address);

            let amount_to_sub = liq_comp_amount
                .checked_neg()
                .ok_or(Error::LiquidateMathError)?;
            s_token_supply = s_token_supply
                .checked_sub(liq_lp_amount)
                .ok_or(Error::LiquidateMathError)?;

            if liq_lp_amount > 0 && liquidator_part_underlying > 0 {
                s_token.burn(who, &liq_lp_amount, &liquidator_part_underlying, liquidator);
            }

            add_stoken_underlying_balance(env, &s_token.address, amount_to_sub)?;

            write_token_total_supply(env, s_token_address, s_token_supply)?;
            write_token_balance(
                env,
                &s_token.address,
                who,
                collat.lp_balance.unwrap() - liq_lp_amount,
            )?;

            recalculate_reserve_data(
                env,
                &collat.asset,
                &collat.reserve,
                s_token_supply,
                debt_token_supply,
            )?;
        } else {
            let who_rwa_balance_before = read_token_balance(env, &collat.asset, who);
            let who_rwa_balance_after = who_rwa_balance_before
                .checked_sub(liq_comp_amount)
                .ok_or(Error::MathOverflowError)?;
            token::Client::new(env, &collat.asset).transfer(
                &env.current_contract_address(),
                liquidator,
                &liquidator_part_underlying,
            );

            write_token_balance(env, &collat.asset, who, who_rwa_balance_after)?;
        }

        if protocol_part_underlying > 0 {
            add_protocol_fee_vault(env, &collat.asset, protocol_part_underlying)?;
        }

        user_configurator.withdraw(
            collat.reserve.get_id(),
            &collat.asset,
            collat.comp_balance == liq_comp_amount,
        )?;

        if full_liquidation {
            continue;
        }

        total_debt_to_cover_in_base += debt_in_base;

        let npv_after = total_collat_disc_after_in_base
            .checked_sub(total_debt_after_in_base)
            .ok_or(Error::LiquidateMathError)?;

        if npv_after.is_positive() {
            break;
        }
    }

    for debt in account_data.liq_debts.ok_or(Error::LiquidateMathError)? {
        if !full_liquidation && total_debt_to_cover_in_base.eq(&0) {
            break;
        }

        if let ReserveType::Fungible(s_token_address, debt_token_address) =
            &debt.reserve.reserve_type
        {
            let debt_comp_in_base =
                price_provider.convert_to_base(&debt.asset, debt.comp_balance)?;

            let (debt_lp_to_burn, debt_comp_to_transfer) =
                if full_liquidation || total_debt_to_cover_in_base >= debt_comp_in_base {
                    total_debt_to_cover_in_base -= debt_comp_in_base;
                    debt_covered_in_base += debt_comp_in_base;

                    user_configurator.repay(debt.reserve.get_id(), true)?;

                    (debt.lp_balance.unwrap(), debt.comp_balance)
                } else {
                    let debt_comp_amount = price_provider.convert_from_base(
                        &debt.asset,
                        total_debt_to_cover_in_base,
                        true,
                    )?; // ceil (convert_from_base_with_ceil)

                    let debt_lp_amount = FixedI128::from_inner(debt.coeff.unwrap())
                        .recip_mul_int(debt_comp_amount)
                        .ok_or(Error::LiquidateMathError)?;

                    debt_covered_in_base += total_debt_to_cover_in_base;
                    total_debt_to_cover_in_base = 0;

                    (debt_lp_amount, debt_comp_amount)
                };

            let underlying_asset = token::Client::new(env, &debt.asset);
            let debt_token = DebtTokenClient::new(env, debt_token_address);

            if debt_comp_to_transfer > 0 {
                underlying_asset.transfer(liquidator, s_token_address, &debt_comp_to_transfer);
            }

            if debt_lp_to_burn > 0 {
                debt_token.burn(who, &debt_lp_to_burn);
            }

            let mut debt_token_supply = read_token_total_supply(env, debt_token_address);
            let s_token_supply = read_token_total_supply(env, s_token_address);

            debt_token_supply = debt_token_supply
                .checked_sub(debt_lp_to_burn)
                .ok_or(Error::LiquidateMathError)?;

            add_stoken_underlying_balance(env, s_token_address, debt_comp_to_transfer)?;
            write_token_total_supply(env, debt_token_address, debt_token_supply)?;
            write_token_balance(
                env,
                &debt_token.address,
                who,
                debt.lp_balance.unwrap() - debt_lp_to_burn,
            )?;

            recalculate_reserve_data(
                env,
                &debt.asset,
                &debt.reserve,
                s_token_supply,
                debt_token_supply,
            )?;
        }
    }

    user_configurator.write();

    Ok((debt_covered_in_base, total_liq_in_base))
}

#[allow(clippy::too_many_arguments)]
fn calc_liq_amount(
    price_provider: &mut PriceProvider,
    collat: &LiquidationAsset,
    hundred_percent: FixedI128,
    discount_percent: FixedI128,
    liq_bonus_percent: FixedI128,
    safe_collat_percent: FixedI128,
    initial_health_percent: FixedI128,
    total_collat_disc_in_base: i128,
    total_debt_in_base: i128,
) -> Result<i128, Error> {
    let safe_collat_in_base = safe_collat_percent
        .mul_int(total_collat_disc_in_base)
        .ok_or(Error::LiquidateMathError)?
        .checked_sub(total_debt_in_base)
        .ok_or(Error::LiquidateMathError)?;

    let safe_discount_percent = discount_percent
        .checked_mul(initial_health_percent)
        .unwrap();

    let safe_discount_percent = discount_percent
        .checked_add(liq_bonus_percent)
        .ok_or(Error::LiquidateMathError)?
        .checked_sub(hundred_percent)
        .ok_or(Error::LiquidateMathError)?
        .checked_sub(safe_discount_percent)
        .ok_or(Error::LiquidateMathError)?;

    let liq_comp_amount =
        price_provider.convert_from_base(&collat.asset, safe_collat_in_base, false)?;

    let liq_comp_amount = safe_discount_percent
        .recip_mul_int(liq_comp_amount)
        .ok_or(Error::LiquidateMathError)?;

    Ok(if liq_comp_amount.is_negative() || liq_comp_amount == 0 {
        collat.comp_balance
    } else {
        collat.comp_balance.min(liq_comp_amount)
    })
}
