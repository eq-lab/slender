use debt_token_interface::DebtTokenClient;
use pool_interface::types::asset_balance::AssetBalance;
use pool_interface::types::error::Error;
use pool_interface::types::reserve_data::ReserveData;
use s_token_interface::STokenClient;
use soroban_sdk::{Address, Env};

use crate::event;
use crate::storage::{
    add_stoken_underlying_balance, read_reserve, read_token_balance, read_token_total_supply,
    write_token_balance, write_token_total_supply,
};
use crate::types::calc_account_data_cache::CalcAccountDataCache;
use crate::types::price_provider::PriceProvider;
use crate::types::user_configurator::UserConfigurator;

use super::account_position::calc_account_data;
use super::utils::get_fungible_lp_tokens::get_fungible_lp_tokens;
use super::utils::rate::get_actual_borrower_accrued_rate;
use super::utils::recalculate_reserve_data::recalculate_reserve_data;
use super::utils::validation::{
    require_active_reserve, require_borrowing_enabled, require_gte_initial_health,
    require_not_in_collateral_asset, require_not_paused, require_positive_amount,
    require_util_cap_not_exceeded,
};

pub fn borrow(env: &Env, who: &Address, asset: &Address, amount: i128) -> Result<(), Error> {
    who.require_auth();

    require_not_paused(env);
    require_positive_amount(env, amount);

    let reserve = read_reserve(env, asset)?;
    require_active_reserve(env, &reserve);
    require_borrowing_enabled(env, &reserve);

    let (s_token_address, debt_token_address) = get_fungible_lp_tokens(&reserve)?;

    let s_token_supply = read_token_total_supply(env, s_token_address);

    let debt_token_supply_after = do_borrow(
        env,
        who,
        asset,
        &reserve,
        read_token_balance(env, s_token_address, who),
        read_token_balance(env, debt_token_address, who),
        s_token_supply,
        read_token_total_supply(env, debt_token_address),
        amount,
        s_token_address,
        debt_token_address,
    )?;

    recalculate_reserve_data(
        env,
        asset,
        &reserve,
        s_token_supply,
        debt_token_supply_after,
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn do_borrow(
    env: &Env,
    who: &Address,
    asset: &Address,
    reserve: &ReserveData,
    who_collat: i128,
    who_debt: i128,
    s_token_supply: i128,
    debt_token_supply: i128,
    amount: i128,
    s_token_address: &Address,
    debt_token_address: &Address,
) -> Result<i128, Error> {
    require_not_in_collateral_asset(env, who_collat);
    require_positive_amount(env, amount);

    let mut price_provider = PriceProvider::new(env)?;
    let amount_in_base = price_provider.convert_to_base(asset, amount)?;

    let mut user_configurator = UserConfigurator::new(env, who, false);
    let user_config = user_configurator.user_config()?;

    let account_data = calc_account_data(
        env,
        who,
        &CalcAccountDataCache {
            mb_who_collat: None,
            mb_who_debt: Some(&AssetBalance::new(debt_token_address.clone(), who_debt)),
            mb_s_token_supply: None,
            mb_debt_token_supply: None,
            mb_s_token_underlying_balance: None, // used only for withdraw
            mb_rwa_balance: None,
        },
        user_config,
        &mut price_provider,
        false,
    )?;

    require_gte_initial_health(env, &account_data, amount_in_base)?;

    let debt_coeff = get_actual_borrower_accrued_rate(env, reserve)?;
    let amount_of_debt_token = debt_coeff
        .recip_mul_int_ceil(amount)
        .ok_or(Error::MathOverflowError)?;

    require_util_cap_not_exceeded(
        env,
        s_token_supply,
        debt_token_supply,
        reserve.configuration.util_cap,
        amount_of_debt_token,
    )?;

    let amount_to_sub = amount.checked_neg().ok_or(Error::MathOverflowError)?;
    let debt_token_supply_after = debt_token_supply
        .checked_add(amount_of_debt_token)
        .ok_or(Error::MathOverflowError)?;
    let who_debt_after = who_debt
        .checked_add(amount_of_debt_token)
        .ok_or(Error::MathOverflowError)?;

    DebtTokenClient::new(env, debt_token_address).mint(who, &amount_of_debt_token);
    STokenClient::new(env, s_token_address).transfer_underlying_to(who, &amount);

    add_stoken_underlying_balance(env, s_token_address, amount_to_sub)?;
    write_token_total_supply(env, debt_token_address, debt_token_supply_after)?;
    write_token_balance(env, debt_token_address, who, who_debt_after)?;

    user_configurator
        .borrow(reserve.get_id(), who_debt == 0)?
        .write();

    event::borrow(env, who, asset, amount);

    Ok(debt_token_supply_after)
}
