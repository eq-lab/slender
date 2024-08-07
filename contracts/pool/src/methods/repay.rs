use debt_token_interface::DebtTokenClient;
use pool_interface::types::asset_balance::AssetBalance;
use pool_interface::types::error::Error;
use pool_interface::types::pool_config::PoolConfig;
use pool_interface::types::reserve_data::ReserveData;
use soroban_sdk::{token, Address, Env};

use crate::storage::{
    read_reserve, read_token_balance, read_token_total_supply, write_token_balance,
    write_token_total_supply,
};
use crate::types::calc_account_data_cache::CalcAccountDataCache;
use crate::types::price_provider::PriceProvider;
use crate::types::user_configurator::UserConfigurator;
use crate::{add_protocol_fee_vault, add_token_balance, event, read_pause_info, read_pool_config};

use super::account_position::calc_account_data;
use super::utils::get_collat_coeff::get_collat_coeff;
use super::utils::rate::get_actual_borrower_accrued_rate;
use super::utils::recalculate_reserve_data::recalculate_reserve_data;
use super::utils::validation::{
    require_active_reserve, require_debt, require_min_position_amounts, require_not_paused,
    require_positive_amount,
};

pub fn repay(env: &Env, who: &Address, asset: &Address, amount: i128) -> Result<(), Error> {
    who.require_auth();

    let pause_info = read_pause_info(env);
    require_not_paused(env, &pause_info);

    require_positive_amount(env, amount);

    let reserve = read_reserve(env, asset)?;
    require_active_reserve(env, &reserve);

    let (s_token_address, debt_token_address) = reserve.get_fungible()?;
    let s_token_supply = read_token_total_supply(env, s_token_address);
    let debt_token_supply = read_token_total_supply(env, debt_token_address);
    let pool_config = read_pool_config(env)?;

    let debt_token_supply_after = do_repay(
        env,
        who,
        asset,
        &reserve,
        &pool_config,
        s_token_supply,
        debt_token_supply,
        s_token_address,
        debt_token_address,
        amount,
    )?;

    recalculate_reserve_data(
        env,
        asset,
        &reserve,
        &pool_config,
        s_token_supply,
        debt_token_supply_after,
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn do_repay(
    env: &Env,
    who: &Address,
    asset: &Address,
    reserve: &ReserveData,
    pool_config: &PoolConfig,
    s_token_supply: i128,
    debt_token_supply: i128,
    s_token_address: &Address,
    debt_token_address: &Address,
    amount: i128,
) -> Result<i128, Error> {
    let mut user_configurator = UserConfigurator::new(env, who, false, None);
    require_debt(env, user_configurator.user_config()?, reserve.get_id());

    let debt_coeff = get_actual_borrower_accrued_rate(env, reserve, pool_config)?;
    let s_token_underlying_balance = read_token_balance(env, asset, s_token_address);

    let collat_coeff = get_collat_coeff(
        env,
        reserve,
        pool_config,
        s_token_supply,
        s_token_underlying_balance,
        debt_token_supply,
    )?;

    let who_debt = read_token_balance(env, debt_token_address, who);
    let borrower_actual_debt = debt_coeff
        .mul_int(who_debt)
        .ok_or(Error::MathOverflowError)?;

    let (borrower_payback_amount, borrower_debt_to_burn, is_repayed) =
        if amount >= borrower_actual_debt {
            (borrower_actual_debt, who_debt, true)
        } else {
            let borrower_debt_to_burn = debt_coeff
                .recip_mul_int(amount)
                .ok_or(Error::MathOverflowError)?;
            (amount, borrower_debt_to_burn, false)
        };

    let treasury_coeff = debt_coeff
        .checked_sub(collat_coeff)
        .ok_or(Error::MathOverflowError)?;
    let treasury_part = treasury_coeff
        .mul_int(borrower_payback_amount)
        .ok_or(Error::MathOverflowError)?;
    let lender_part = borrower_payback_amount
        .checked_sub(treasury_part)
        .ok_or(Error::MathOverflowError)?;

    let debt_token_supply_after = debt_token_supply
        .checked_sub(borrower_debt_to_burn)
        .ok_or(Error::MathOverflowError)?;
    let who_debt_after = who_debt
        .checked_sub(borrower_debt_to_burn)
        .ok_or(Error::MathOverflowError)?;
    let s_token_underlying_after = s_token_underlying_balance
        .checked_add(lender_part)
        .ok_or(Error::MathOverflowError)?;

    user_configurator.repay(reserve.get_id(), is_repayed)?;

    let account_data = calc_account_data(
        env,
        who,
        &CalcAccountDataCache {
            mb_who_collat: None,
            mb_who_debt: Some(&AssetBalance::new(
                debt_token_address.clone(),
                who_debt_after,
            )),
            mb_s_token_supply: Some(&AssetBalance::new(s_token_address.clone(), s_token_supply)),
            mb_debt_token_supply: Some(&AssetBalance::new(
                debt_token_address.clone(),
                debt_token_supply_after,
            )),
            mb_s_token_underlying_balance: Some(&AssetBalance::new(
                s_token_address.clone(),
                s_token_underlying_after,
            )),
            mb_rwa_balance: None,
        },
        pool_config,
        user_configurator.user_config()?,
        &mut PriceProvider::new(env, pool_config)?,
        false,
    )?;

    require_min_position_amounts(env, &account_data, pool_config)?;

    let underlying_asset = token::Client::new(env, asset);
    let debt_token = DebtTokenClient::new(env, debt_token_address);

    underlying_asset.transfer(who, s_token_address, &borrower_payback_amount);
    add_protocol_fee_vault(env, asset, treasury_part)?;
    debt_token.burn(who, &borrower_debt_to_burn);

    add_token_balance(env, asset, s_token_address, lender_part)?;
    write_token_total_supply(env, debt_token_address, debt_token_supply_after)?;
    write_token_balance(env, debt_token_address, who, who_debt_after)?;

    event::repay(env, who, asset, borrower_payback_amount);

    user_configurator.write();

    Ok(debt_token_supply_after)
}
