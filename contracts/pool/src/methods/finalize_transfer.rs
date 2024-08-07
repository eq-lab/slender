use pool_interface::types::asset_balance::AssetBalance;
use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env};

use crate::storage::{read_reserve, read_token_total_supply, write_token_balance};
use crate::types::calc_account_data_cache::CalcAccountDataCache;
use crate::types::price_provider::PriceProvider;
use crate::types::user_configurator::UserConfigurator;
use crate::{read_pause_info, read_pool_config};

use super::account_position::calc_account_data;
use super::utils::validation::{
    require_active_reserve, require_gte_initial_health, require_min_position_amounts,
    require_not_in_grace_period, require_not_paused, require_zero_debt,
};

#[allow(clippy::too_many_arguments)]
pub fn finalize_transfer(
    env: &Env,
    asset: &Address,
    from: &Address,
    to: &Address,
    amount: i128,
    balance_from_before: i128,
    balance_to_before: i128,
    s_token_supply: i128,
) -> Result<(), Error> {
    let pause_info = read_pause_info(env);
    require_not_paused(env, &pause_info);
    require_not_in_grace_period(env, &pause_info);

    let reserve: pool_interface::types::reserve_data::ReserveData = read_reserve(env, asset)?;
    let reserve_id = reserve.get_id();
    require_active_reserve(env, &reserve);
    let (s_token_address, debt_token_address) = reserve.get_fungible()?;
    s_token_address.require_auth();

    let pool_config = read_pool_config(env)?;
    let mut to_configurator =
        UserConfigurator::new(env, to, true, Some(pool_config.user_assets_limit));
    let to_config = to_configurator.user_config()?;

    require_zero_debt(env, to_config, reserve.get_id());

    let balance_from_after = balance_from_before
        .checked_sub(amount)
        .ok_or(Error::InvalidAmount)?;

    let mut from_configurator = UserConfigurator::new(env, from, false, None);
    let is_using_as_collateral = from_configurator
        .user_config()?
        .is_using_as_collateral(env, reserve.get_id());
    let is_borrowing_any = from_configurator.user_config()?.is_borrowing_any();

    if is_borrowing_any && is_using_as_collateral {
        from_configurator.withdraw(reserve_id, asset, balance_from_after == 0)?;

        let from_account_data = calc_account_data(
            env,
            from,
            &CalcAccountDataCache {
                mb_who_collat: Some(&AssetBalance::new(
                    s_token_address.clone(),
                    balance_from_after,
                )),
                mb_who_debt: None,
                mb_s_token_supply: Some(&AssetBalance::new(
                    s_token_address.clone(),
                    s_token_supply,
                )),
                mb_debt_token_supply: Some(&AssetBalance::new(
                    debt_token_address.clone(),
                    read_token_total_supply(env, debt_token_address),
                )),
                mb_s_token_underlying_balance: None,
                mb_rwa_balance: None,
            },
            &pool_config,
            from_configurator.user_config()?,
            &mut PriceProvider::new(env, &pool_config)?,
            false,
        )?;

        require_min_position_amounts(env, &from_account_data, &pool_config)?;
        require_gte_initial_health(env, &from_account_data, &pool_config)?;
    }

    if from != to {
        let balance_to_after = balance_to_before
            .checked_add(amount)
            .ok_or(Error::InvalidAmount)?;

        write_token_balance(env, s_token_address, from, balance_from_after)?;
        write_token_balance(env, s_token_address, to, balance_to_after)?;

        let is_to_deposit = balance_to_before == 0 && amount != 0;

        from_configurator
            .withdraw(reserve_id, asset, balance_from_after == 0)?
            .write();

        to_configurator
            .deposit(reserve_id, asset, is_to_deposit)?
            .write();
    }

    Ok(())
}
