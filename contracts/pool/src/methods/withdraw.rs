use debt_token_interface::DebtTokenClient;
use pool_interface::types::asset_balance::AssetBalance;
use pool_interface::types::error::Error;
use s_token_interface::STokenClient;
use soroban_sdk::{assert_with_error, Address, Env};

use crate::event;
use crate::methods::account_position::CalcAccountDataCache;
use crate::storage::{
    add_stoken_underlying_balance, read_reserve, read_token_total_supply, write_token_total_supply,
};
use crate::types::user_configurator::UserConfigurator;

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

    let s_token = STokenClient::new(env, &reserve.s_token_address);
    let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
    let s_token_supply = read_token_total_supply(env, &reserve.s_token_address);
    let debt_token_supply = read_token_total_supply(env, &reserve.debt_token_address);

    let collat_coeff = get_collat_coeff(env, &reserve, s_token_supply, debt_token_supply)?;

    let collat_balance = s_token.balance(who);
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

    let mut user_configurator = UserConfigurator::new(env, who, false);
    let user_config = user_configurator.user_config()?;
    let collat_balance_after = collat_balance
        .checked_sub(s_token_to_burn)
        .ok_or(Error::InvalidAmount)?;
    let s_token_supply_after = s_token_supply
        .checked_sub(s_token_to_burn)
        .ok_or(Error::InvalidAmount)?;

    if user_config.is_borrowing_any() && user_config.is_using_as_collateral(env, reserve.get_id()) {
        let account_data = calc_account_data(
            env,
            who,
            CalcAccountDataCache {
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
                    debt_token.address,
                    debt_token_supply,
                )),
            },
            user_config,
            false,
        )?;
        require_good_position(env, &account_data);
    }
    let amount_to_sub = underlying_to_withdraw
        .checked_neg()
        .ok_or(Error::MathOverflowError)?;

    s_token.burn(who, &s_token_to_burn, &underlying_to_withdraw, to);

    write_token_total_supply(env, &s_token.address, s_token_supply_after)?;
    add_stoken_underlying_balance(env, &s_token.address, amount_to_sub)?;

    let is_full_withdraw = underlying_to_withdraw == underlying_balance;
    user_configurator
        .withdraw(reserve.get_id(), asset, is_full_withdraw)?
        .write();

    event::withdraw(env, who, asset, to, underlying_to_withdraw);

    recalculate_reserve_data(
        env,
        asset,
        &reserve,
        s_token_supply_after,
        debt_token_supply,
    )?;

    Ok(())
}
