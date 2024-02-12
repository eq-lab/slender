use pool_interface::types::asset_balance::AssetBalance;
use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env};

use crate::storage::{read_reserve, read_token_total_supply, write_token_balance};
use crate::types::calc_account_data_cache::CalcAccountDataCache;
use crate::types::price_provider::PriceProvider;
use crate::types::user_configurator::UserConfigurator;

use super::account_position::calc_account_data;
use super::utils::get_fungible_lp_tokens::get_fungible_lp_tokens;
use super::utils::validation::{
    require_active_reserve, require_good_position, require_not_paused, require_zero_debt,
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
    require_not_paused(env);

    let reserve = read_reserve(env, asset)?;
    require_active_reserve(env, &reserve);
    let (s_token_address, debt_token_address) = get_fungible_lp_tokens(&reserve)?;
    s_token_address.require_auth();

    let mut to_configurator = UserConfigurator::new(env, to, true);
    let to_config = to_configurator.user_config()?;

    require_zero_debt(env, to_config, reserve.get_id());

    let balance_from_after = balance_from_before
        .checked_sub(amount)
        .ok_or(Error::InvalidAmount)?;

    let mut from_configurator = UserConfigurator::new(env, from, false);
    let from_config = from_configurator.user_config()?;

    if from_config.is_borrowing_any() && from_config.is_using_as_collateral(env, reserve.get_id()) {
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
            from_config,
            &mut PriceProvider::new(env)?,
            false,
        )?;

        require_good_position(env, &from_account_data);
    }

    if from != to {
        let balance_to_after = balance_to_before
            .checked_add(amount)
            .ok_or(Error::InvalidAmount)?;

        write_token_balance(env, s_token_address, from, balance_from_after)?;
        write_token_balance(env, s_token_address, to, balance_to_after)?;

        let reserve_id = reserve.get_id();
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
