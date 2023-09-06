use common::FixedI128;
use debt_token_interface::DebtTokenClient;
use pool_interface::types::error::Error;
use pool_interface::types::reserve_data::ReserveData;
use s_token_interface::STokenClient;
use soroban_sdk::{token, Address, Env};

use crate::event;
use crate::storage::{add_stoken_underlying_balance, read_reserve, read_treasury};
use crate::types::user_configurator::UserConfigurator;

use super::collat_coeff::get_collat_coeff;
use super::init_reserve::recalculate_reserve_data;
use super::rate::get_actual_borrower_accrued_rate;
use super::validation::{
    require_active_reserve, require_debt, require_not_paused, require_positive_amount,
};

#[cfg(not(feature = "exceeded-limit-fix"))]
pub fn repay(env: &Env, who: &Address, asset: &Address, amount: i128) -> Result<(), Error> {
    who.require_auth();

    require_not_paused(env);
    require_positive_amount(env, amount);

    let reserve = read_reserve(env, asset)?;
    require_active_reserve(env, &reserve);

    let mut user_configurator = UserConfigurator::new(env, who, false);
    let user_config = user_configurator.user_config()?;
    require_debt(env, user_config, reserve.get_id());

    let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
    let s_token = STokenClient::new(env, &reserve.s_token_address);
    let s_token_supply = s_token.total_supply();
    let debt_token_supply = debt_token.total_supply();

    let debt_coeff = get_actual_borrower_accrued_rate(env, &reserve)?;
    let collat_coeff = get_collat_coeff(env, &reserve, s_token_supply, debt_token_supply)?;

    let (is_repayed, debt_token_supply_after) = do_repay(
        env,
        who,
        asset,
        &reserve,
        collat_coeff,
        debt_coeff,
        debt_token_supply,
        debt_token.balance(who),
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

#[cfg(feature = "exceeded-limit-fix")]
pub fn repay(
    env: &Env,
    who: &Address,
    asset: &Address,
    amount: i128,
) -> Result<Vec<MintBurn>, Error> {
    who.require_auth();

    require_not_paused(env);
    require_positive_amount(env, amount);

    let reserve = read_reserve(env, asset)?;
    require_active_reserve(env, &reserve);

    let mut user_configurator = UserConfigurator::new(env, who, false);
    let user_config = user_configurator.user_config()?;
    require_debt(env, user_config, reserve.get_id());

    let s_token_supply = read_token_total_supply(env, &reserve.s_token_address);
    let debt_token_supply = read_token_total_supply(env, &reserve.debt_token_address);

    let debt_coeff = get_actual_borrower_accrued_rate(env, &reserve)?;
    let collat_coeff = get_collat_coeff(env, &reserve, s_token_supply, debt_token_supply)?;

    let (is_repayed, debt_token_supply_after, mints_burns) = do_repay(
        env,
        who,
        asset,
        &reserve,
        collat_coeff,
        debt_coeff,
        debt_token_supply,
        read_token_balance(env, &reserve.debt_token_address, who),
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

    Ok(mints_burns)
}

/// Returns
/// bool: the flag indicating the debt is fully repayed
/// i128: total debt after repayment
#[allow(clippy::too_many_arguments)]
#[cfg(not(feature = "exceeded-limit-fix"))]
pub fn do_repay(
    env: &Env,
    who: &Address,
    asset: &Address,
    reserve: &ReserveData,
    collat_coeff: FixedI128,
    debt_coeff: FixedI128,
    debt_token_supply: i128,
    who_debt: i128,
    amount: i128,
) -> Result<(bool, i128), Error> {
    let borrower_actual_debt = debt_coeff
        .mul_int(who_debt)
        .ok_or(Error::MathOverflowError)?;

    let (borrower_payback_amount, borrower_debt_to_burn, is_repayed) =
        if amount >= borrower_actual_debt {
            // To avoid dust in debt_token borrower balance in case of full repayment
            (borrower_actual_debt, who_debt, true)
        } else {
            let borrower_debt_to_burn = debt_coeff
                .recip_mul_int(amount)
                .ok_or(Error::MathOverflowError)?;
            (amount, borrower_debt_to_burn, false)
        };

    let lender_part = collat_coeff
        .mul_int(borrower_debt_to_burn)
        .ok_or(Error::MathOverflowError)?;
    let treasury_part = borrower_payback_amount
        .checked_sub(lender_part)
        .ok_or(Error::MathOverflowError)?;
    let debt_token_supply_after = debt_token_supply
        .checked_sub(borrower_debt_to_burn)
        .ok_or(Error::MathOverflowError)?;

    let treasury_address = read_treasury(env);

    let underlying_asset = token::Client::new(env, asset);

    underlying_asset.transfer(who, &reserve.s_token_address, &lender_part);
    add_stoken_underlying_balance(env, &reserve.s_token_address, lender_part)?;
    underlying_asset.transfer(who, &treasury_address, &treasury_part);
    DebtTokenClient::new(env, &reserve.debt_token_address).burn(who, &borrower_debt_to_burn);

    event::repay(env, who, asset, borrower_payback_amount);

    Ok((is_repayed, debt_token_supply_after))
}

/// Returns
/// bool: the flag indicating the debt is fully repayed
/// i128: total debt after repayment
#[allow(clippy::too_many_arguments)]
#[cfg(feature = "exceeded-limit-fix")]
pub fn do_repay(
    env: &Env,
    who: &Address,
    asset: &Address,
    reserve: &ReserveData,
    collat_coeff: FixedI128,
    debt_coeff: FixedI128,
    debt_token_supply: i128,
    who_debt: i128,
    amount: i128,
) -> Result<(bool, i128, Vec<MintBurn>), Error> {
    let borrower_actual_debt = debt_coeff
        .mul_int(who_debt)
        .ok_or(Error::MathOverflowError)?;

    let (borrower_payback_amount, borrower_debt_to_burn, is_repayed) =
        if amount >= borrower_actual_debt {
            // To avoid dust in debt_token borrower balance in case of full repayment
            (borrower_actual_debt, who_debt, true)
        } else {
            let borrower_debt_to_burn = debt_coeff
                .recip_mul_int(amount)
                .ok_or(Error::MathOverflowError)?;
            (amount, borrower_debt_to_burn, false)
        };

    let lender_part = collat_coeff
        .mul_int(borrower_debt_to_burn)
        .ok_or(Error::MathOverflowError)?;
    let treasury_part = borrower_payback_amount
        .checked_sub(lender_part)
        .ok_or(Error::MathOverflowError)?;
    let debt_token_supply_after = debt_token_supply
        .checked_sub(borrower_debt_to_burn)
        .ok_or(Error::MathOverflowError)?;

    let treasury_address = read_treasury(env);

    let debt_to_sub = borrower_debt_to_burn
        .checked_neg()
        .ok_or(Error::MathOverflowError)?;

    add_token_balance(env, &reserve.debt_token_address, who, debt_to_sub)?;
    add_token_total_supply(env, &reserve.debt_token_address, debt_to_sub)?;
    add_stoken_underlying_balance(env, &reserve.s_token_address, lender_part)?;

    let mint_burn_1 = MintBurn::new(
        AssetBalance::new(reserve.debt_token_address.clone(), borrower_debt_to_burn),
        false,
        who.clone(),
    );
    let mint_burn_2 = MintBurn::new(
        AssetBalance::new(asset.clone(), lender_part),
        false,
        who.clone(),
    );
    let mint_burn_3 = MintBurn::new(
        AssetBalance::new(asset.clone(), lender_part),
        true,
        reserve.s_token_address.clone(),
    );
    let mint_burn_4 = MintBurn::new(
        AssetBalance::new(asset.clone(), treasury_part),
        false,
        who.clone(),
    );
    let mint_burn_5 = MintBurn::new(
        AssetBalance::new(asset.clone(), treasury_part),
        true,
        treasury_address.clone(),
    );

    event::repay(env, who, asset, borrower_payback_amount);

    Ok((
        is_repayed,
        debt_token_supply_after,
        vec![
            env,
            mint_burn_1,
            mint_burn_2,
            mint_burn_3,
            mint_burn_4,
            mint_burn_5,
        ],
    ))
}
