use crate::event;
use crate::storage::{
    add_stoken_underlying_balance, read_reserve, read_stoken_underlying_balance,
    read_token_balance, read_token_total_supply, write_token_balance, write_token_total_supply,
};
use crate::types::calc_account_data_cache::CalcAccountDataCache;
use crate::types::price_provider::PriceProvider;
use crate::types::user_configurator::UserConfigurator;
use pool_interface::types::asset_balance::AssetBalance;
use pool_interface::types::error::Error;
use pool_interface::types::reserve_type::ReserveType;
use s_token_interface::STokenClient;
use soroban_sdk::{assert_with_error, token, Address, Env};

use super::account_position::calc_account_data;
use super::utils::get_collat_coeff::get_collat_coeff;
use super::utils::recalculate_reserve_data::recalculate_reserve_data;
use super::utils::validation::{
    require_active_reserve, require_good_position, require_not_paused, require_positive_amount,
};

pub fn withdraw(
    env: &Env,
    who: &Address,
    asset: &Address,
    amount: i128,
    to: &Address,
) -> Result<(), Error> {
    who.require_auth();

    require_not_paused(env);
    require_positive_amount(env, amount);

    let reserve = read_reserve(env, asset)?;
    require_active_reserve(env, &reserve);
    let mut user_configurator = UserConfigurator::new(env, who, false);
    let user_config = user_configurator.user_config()?;

    let (withdraw_amount, is_full_withdraw) =
        if let ReserveType::Fungible(s_token_address, debt_token_address) = &reserve.reserve_type {
            let s_token_supply = read_token_total_supply(env, s_token_address);
            let debt_token_supply = read_token_total_supply(env, debt_token_address);
            let collat_coeff = get_collat_coeff(
                env,
                &reserve,
                s_token_supply,
                read_stoken_underlying_balance(env, s_token_address),
                debt_token_supply,
            )?;

            let s_token = STokenClient::new(env, s_token_address);

            let collat_balance = read_token_balance(env, s_token_address, who);
            let underlying_balance = collat_coeff
                .mul_int(collat_balance)
                .ok_or(Error::MathOverflowError)?;

            let (underlying_to_withdraw, s_token_to_burn) = if amount >= underlying_balance {
                (underlying_balance, collat_balance)
            } else {
                let s_token_to_burn = collat_coeff
                    .recip_mul_int(amount)
                    .ok_or(Error::MathOverflowError)?;
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
            let s_token_underlying_after = read_stoken_underlying_balance(env, s_token_address)
                .checked_sub(underlying_to_withdraw)
                .ok_or(Error::MathOverflowError)?;

            if user_config.is_borrowing_any()
                && user_config.is_using_as_collateral(env, reserve.get_id())
            {
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
                    user_config,
                    &mut PriceProvider::new(env)?,
                    false,
                )?;
                // TODO: do we need to check for initial_health?
                require_good_position(env, &account_data);
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
                s_token_supply_after,
                debt_token_supply,
            )?;

            (
                underlying_to_withdraw,
                underlying_to_withdraw == underlying_balance,
            )
        } else {
            let rwa_balance = read_token_balance(env, asset, who);

            let withdraw_amount = amount.min(rwa_balance);
            let rwa_balance_after = rwa_balance - withdraw_amount;

            if user_config.is_borrowing_any()
                && user_config.is_using_as_collateral(env, reserve.get_id())
            {
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
                    user_config,
                    &mut PriceProvider::new(env)?,
                    false,
                )?;

                // TODO: do we need to check for initial_health?
                require_good_position(env, &account_data);
            }
            token::Client::new(env, asset).transfer(
                &env.current_contract_address(),
                who,
                &withdraw_amount,
            );

            write_token_balance(env, asset, who, rwa_balance_after)?;

            (withdraw_amount, rwa_balance_after == 0)
        };

    user_configurator
        .withdraw(reserve.get_id(), asset, is_full_withdraw)?
        .write();

    event::withdraw(env, who, asset, to, withdraw_amount);

    Ok(())
}
