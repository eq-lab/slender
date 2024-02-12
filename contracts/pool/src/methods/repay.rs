use common::FixedI128;
use debt_token_interface::DebtTokenClient;
use pool_interface::types::error::Error;
use soroban_sdk::{token, Address, Env};

use crate::event;
use crate::storage::{
    add_stoken_underlying_balance, read_reserve, read_stoken_underlying_balance,
    read_token_balance, read_token_total_supply, read_treasury, write_token_balance,
    write_token_total_supply,
};
use crate::types::user_configurator::UserConfigurator;

use super::utils::get_collat_coeff::get_collat_coeff;
use super::utils::get_fungible_lp_tokens::get_fungible_lp_tokens;
use super::utils::rate::get_actual_borrower_accrued_rate;
use super::utils::recalculate_reserve_data::recalculate_reserve_data;
use super::utils::validation::{
    require_active_reserve, require_debt, require_not_paused, require_positive_amount,
};

pub fn repay(env: &Env, who: &Address, asset: &Address, amount: i128) -> Result<(), Error> {
    who.require_auth();

    require_not_paused(env);
    require_positive_amount(env, amount);

    let reserve = read_reserve(env, asset)?;
    require_active_reserve(env, &reserve);

    let (s_token_address, debt_token_address) = get_fungible_lp_tokens(&reserve)?;
    let mut user_configurator = UserConfigurator::new(env, who, false);
    let user_config = user_configurator.user_config()?;
    require_debt(env, user_config, reserve.get_id());

    let s_token_supply = read_token_total_supply(env, s_token_address);
    let debt_token_supply = read_token_total_supply(env, debt_token_address);

    let debt_coeff = get_actual_borrower_accrued_rate(env, &reserve)?;
    let collat_coeff = get_collat_coeff(
        env,
        &reserve,
        s_token_supply,
        read_stoken_underlying_balance(env, s_token_address),
        debt_token_supply,
    )?;

    let (is_repayed, debt_token_supply_after) = do_repay(
        env,
        who,
        asset,
        s_token_address,
        debt_token_address,
        collat_coeff,
        debt_coeff,
        debt_token_supply,
        amount,
    )?;

    user_configurator
        .repay(reserve.get_id(), is_repayed)?
        .write();

    recalculate_reserve_data(
        env,
        asset,
        &reserve,
        s_token_supply,
        debt_token_supply_after,
    )?;

    Ok(())
}

/// Returns
/// bool: the flag indicating the debt is fully repayed
/// i128: total debt after repayment
#[allow(clippy::too_many_arguments)]
pub fn do_repay(
    env: &Env,
    who: &Address,
    asset: &Address,
    s_token_address: &Address,
    debt_token_address: &Address,
    collat_coeff: FixedI128,
    debt_coeff: FixedI128,
    debt_token_supply: i128,
    amount: i128,
) -> Result<(bool, i128), Error> {
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

    let treasury_address = read_treasury(env);

    let underlying_asset = token::Client::new(env, asset);
    let debt_token = DebtTokenClient::new(env, debt_token_address);

    underlying_asset.transfer(who, s_token_address, &lender_part);
    underlying_asset.transfer(who, &treasury_address, &treasury_part);
    debt_token.burn(who, &borrower_debt_to_burn);

    add_stoken_underlying_balance(env, s_token_address, lender_part)?;
    write_token_total_supply(env, debt_token_address, debt_token_supply_after)?;
    write_token_balance(env, debt_token_address, who, who_debt_after)?;

    event::repay(env, who, asset, borrower_payback_amount);

    Ok((is_repayed, debt_token_supply_after))
}
