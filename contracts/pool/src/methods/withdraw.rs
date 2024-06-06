use crate::methods::utils::get_collat_coeff::get_compounded_amount;
use crate::methods::utils::get_collat_coeff::get_lp_amount;
use crate::methods::utils::validation::require_gte_initial_health;
use crate::read_pool_config;
use crate::storage::{
    add_stoken_underlying_balance, read_reserve, read_stoken_underlying_balance,
    read_token_balance, read_token_total_supply, write_token_balance, write_token_total_supply,
};
use crate::types::calc_account_data_cache::CalcAccountDataCache;
use crate::types::price_provider::PriceProvider;
use crate::types::user_configurator::UserConfigurator;
use crate::{event, read_pause_info};
use pool_interface::types::asset_balance::AssetBalance;
use pool_interface::types::error::Error;
use pool_interface::types::reserve_type::ReserveType;
use s_token_interface::STokenClient;
use soroban_sdk::{assert_with_error, token, Address, Env};

use super::account_position::calc_account_data;
use super::utils::recalculate_reserve_data::recalculate_reserve_data;
use super::utils::validation::{
    require_active_reserve, require_min_position_amounts, require_not_in_grace_period,
    require_not_paused, require_positive_amount,
};

pub fn withdraw(
    env: &Env,
    who: &Address,
    asset: &Address,
    amount: i128,
    to: &Address,
) -> Result<(), Error> {
    who.require_auth();

    let pause_info = read_pause_info(env)?;
    require_not_paused(env, &pause_info);
    require_not_in_grace_period(env, &pause_info);

    require_positive_amount(env, amount);

    let reserve = read_reserve(env, asset)?;
    require_active_reserve(env, &reserve);
    let mut user_configurator = UserConfigurator::new(env, who, false, None);

    let pool_config = read_pool_config(env)?;

    let withdraw_amount =
        if let ReserveType::Fungible(s_token_address, debt_token_address) = &reserve.reserve_type {
            let s_token_supply = read_token_total_supply(env, s_token_address);
            let debt_token_supply = read_token_total_supply(env, debt_token_address);

            let s_token = STokenClient::new(env, s_token_address);

            let collat_balance = read_token_balance(env, s_token_address, who);
            let stoken_underlying_balance = read_stoken_underlying_balance(env, s_token_address);

            let underlying_balance = get_compounded_amount(
                env,
                &reserve,
                &pool_config,
                s_token_supply,
                stoken_underlying_balance,
                debt_token_supply,
                collat_balance,
            )?;

            let (underlying_to_withdraw, s_token_to_burn) = if amount >= underlying_balance {
                (underlying_balance, collat_balance)
            } else {
                let s_token_to_burn = get_lp_amount(
                    env,
                    &reserve,
                    &pool_config,
                    s_token_supply,
                    stoken_underlying_balance,
                    debt_token_supply,
                    amount,
                    true,
                )?;

                (amount, s_token_to_burn)
            };

            assert_with_error!(
                env,
                underlying_to_withdraw <= underlying_balance,
                Error::NotEnoughAvailableUserBalance
            );

            let collat_balance_after = collat_balance
                .checked_sub(s_token_to_burn)
                .ok_or(Error::InvalidAmount)?;
            let s_token_supply_after = s_token_supply
                .checked_sub(s_token_to_burn)
                .ok_or(Error::InvalidAmount)?;
            let s_token_underlying_after = stoken_underlying_balance
                .checked_sub(underlying_to_withdraw)
                .ok_or(Error::MathOverflowError)?;

            user_configurator.withdraw(reserve.get_id(), asset, collat_balance_after == 0)?;

            let is_borrowing_any = user_configurator.user_config()?.is_borrowing_any();

            if is_borrowing_any {
                let account_data = calc_account_data(
                    env,
                    who,
                    &CalcAccountDataCache {
                        mb_who_collat: Some(&AssetBalance::new(
                            s_token.address.clone(),
                            collat_balance_after,
                        )),
                        mb_who_debt: None,
                        mb_s_token_supply: Some(&AssetBalance::new(
                            s_token.address.clone(),
                            s_token_supply_after,
                        )),
                        mb_debt_token_supply: Some(&AssetBalance::new(
                            debt_token_address.clone(),
                            debt_token_supply,
                        )),
                        mb_s_token_underlying_balance: Some(&AssetBalance::new(
                            s_token_address.clone(),
                            s_token_underlying_after,
                        )),
                        mb_rwa_balance: None,
                    },
                    &pool_config,
                    user_configurator.user_config()?,
                    &mut PriceProvider::new(env, &pool_config)?,
                    false,
                )?;

                require_min_position_amounts(env, &account_data, &pool_config)?;
                // account data calculation takes into account the decrease of collateral
                require_gte_initial_health(env, &account_data, &pool_config)?;
            }

            let amount_to_sub = underlying_to_withdraw
                .checked_neg()
                .ok_or(Error::MathOverflowError)?;

            s_token.burn(who, &s_token_to_burn, &underlying_to_withdraw, to);

            add_stoken_underlying_balance(env, &s_token.address, amount_to_sub)?;
            write_token_total_supply(env, &s_token.address, s_token_supply_after)?;
            write_token_balance(env, &s_token.address, who, collat_balance_after)?;

            recalculate_reserve_data(
                env,
                asset,
                &reserve,
                &pool_config,
                s_token_supply_after,
                debt_token_supply,
            )?;

            underlying_to_withdraw
        } else {
            let rwa_balance = read_token_balance(env, asset, who);

            let withdraw_amount = amount.min(rwa_balance);
            let rwa_balance_after = rwa_balance - withdraw_amount;

            user_configurator.withdraw(reserve.get_id(), asset, rwa_balance_after == 0)?;

            let is_borrowing_any = user_configurator.user_config()?.is_borrowing_any();

            if is_borrowing_any {
                let account_data = calc_account_data(
                    env,
                    who,
                    &CalcAccountDataCache {
                        mb_who_collat: None,
                        mb_who_debt: None,
                        mb_s_token_supply: None,
                        mb_debt_token_supply: None,
                        mb_s_token_underlying_balance: None,
                        mb_rwa_balance: Some(&AssetBalance::new(asset.clone(), rwa_balance_after)),
                    },
                    &pool_config,
                    user_configurator.user_config()?,
                    &mut PriceProvider::new(env, &pool_config)?,
                    false,
                )?;

                require_min_position_amounts(env, &account_data, &pool_config)?;
                // account data calculation takes into account the decrease of collateral
                require_gte_initial_health(env, &account_data, &pool_config)?;
            }

            token::Client::new(env, asset).transfer(
                &env.current_contract_address(),
                who,
                &withdraw_amount,
            );

            write_token_balance(env, asset, who, rwa_balance_after)?;

            withdraw_amount
        };

    user_configurator.write();

    event::withdraw(env, who, asset, to, withdraw_amount);

    Ok(())
}
