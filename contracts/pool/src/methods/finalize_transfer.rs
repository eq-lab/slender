use debt_token_interface::DebtTokenClient;
use pool_interface::types::asset_balance::AssetBalance;
use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env};

use crate::storage::read_reserve;
use crate::types::user_configurator::UserConfigurator;

use super::account_position::{calc_account_data, CalcAccountDataCache};
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
    reserve.s_token_address.require_auth();

    let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
    let mut to_configurator = UserConfigurator::new(env, to, true);
    let to_config = to_configurator.user_config()?;

    require_zero_debt(env, to_config, reserve.get_id());

    let debt_token_supply = debt_token.total_supply();

    let balance_from_after = balance_from_before
        .checked_sub(amount)
        .ok_or(Error::InvalidAmount)?;

    let mut from_configurator = UserConfigurator::new(env, from, false);
    let from_config = from_configurator.user_config()?;

    if from_config.is_borrowing_any() && from_config.is_using_as_collateral(env, reserve.get_id()) {
        let from_account_data = calc_account_data(
            env,
            from,
            CalcAccountDataCache {
                mb_who_collat: Some(&AssetBalance::new(
                    reserve.s_token_address.clone(),
                    balance_from_after,
                )),
                mb_who_debt: None,
                mb_s_token_supply: Some(&AssetBalance::new(
                    reserve.s_token_address.clone(),
                    s_token_supply,
                )),
                mb_debt_token_supply: Some(&AssetBalance::new(
                    reserve.debt_token_address.clone(),
                    debt_token_supply,
                )),
            },
            from_config,
            false,
        )?;

        require_good_position(env, &from_account_data);
    }

    if from != to {
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
