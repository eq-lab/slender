use common::FixedI128;
use pool_interface::types::asset_balance::AssetBalance;
use pool_interface::types::error::Error;
use pool_interface::types::mint_burn::MintBurn;
use pool_interface::types::reserve_data::ReserveData;
use soroban_sdk::{vec, Address, Env, Vec};

use crate::event;
use crate::methods::utils::get_collat_coeff::get_collat_coeff;
use crate::methods::utils::rate::get_actual_borrower_accrued_rate;
use crate::methods::utils::recalculate_reserve_data::recalculate_reserve_data;
use crate::methods::utils::validation::{
    require_active_reserve, require_debt, require_not_paused, require_positive_amount,
};
use crate::storage::{
    add_stoken_underlying_balance, add_token_balance, read_reserve, read_token_balance,
    read_token_total_supply, read_treasury, write_token_total_supply,
};
use crate::types::user_configurator::UserConfigurator;

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
    write_token_total_supply(env, &reserve.debt_token_address, debt_token_supply_after)?;
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
