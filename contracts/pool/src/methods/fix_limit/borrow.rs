use common::FixedI128;
use pool_interface::types::asset_balance::AssetBalance;
use pool_interface::types::error::Error;
use pool_interface::types::mint_burn::MintBurn;
use soroban_sdk::{assert_with_error, vec, Address, Env, Vec};

use crate::event;
use crate::methods::fix_limit::account_position::{calc_account_data, CalcAccountDataCache};
use crate::methods::utils::rate::get_actual_borrower_accrued_rate;
use crate::methods::utils::recalculate_reserve_data::recalculate_reserve_data;
use crate::methods::utils::validation::{
    require_active_reserve, require_borrowing_enabled, require_not_in_collateral_asset,
    require_not_paused, require_positive_amount, require_util_cap_not_exceeded,
};
use crate::storage::{
    add_stoken_underlying_balance, add_token_balance, read_price, read_reserve, read_token_balance,
    read_token_total_supply, write_token_total_supply,
};
use crate::types::user_configurator::UserConfigurator;

pub fn borrow(
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
    require_borrowing_enabled(env, &reserve);

    let collat_balance = read_token_balance(env, &reserve.s_token_address, who);

    require_not_in_collateral_asset(env, collat_balance);

    let s_token_supply = read_token_total_supply(env, &reserve.s_token_address);
    let debt_token_supply = read_token_total_supply(env, &reserve.debt_token_address);

    let asset_price = FixedI128::from_inner(read_price(env, asset));
    let amount_in_xlm = asset_price
        .mul_int(amount)
        .ok_or(Error::ValidateBorrowMathError)?;
    require_positive_amount(env, amount_in_xlm);

    let mut user_configurator = UserConfigurator::new(env, who, false);
    let user_config = user_configurator.user_config()?;
    let debt_balance = read_token_balance(env, &reserve.debt_token_address, who);

    let account_data = calc_account_data(
        env,
        who,
        CalcAccountDataCache {
            mb_who_collat: None,
            mb_who_debt: Some(&AssetBalance::new(
                reserve.debt_token_address.clone(),
                debt_balance,
            )),
            mb_s_token_supply: None,
            mb_debt_token_supply: None,
        },
        user_config,
        false,
    )?;

    assert_with_error!(
        env,
        account_data.npv >= amount_in_xlm,
        Error::CollateralNotCoverNewBorrow
    );

    let debt_coeff = get_actual_borrower_accrued_rate(env, &reserve)?;
    let amount_of_debt_token = debt_coeff
        .recip_mul_int(amount)
        .ok_or(Error::MathOverflowError)?;
    let util_cap = reserve.configuration.util_cap;

    require_util_cap_not_exceeded(
        env,
        s_token_supply,
        debt_token_supply,
        util_cap,
        amount_of_debt_token,
    )?;

    let debt_token_supply_after = debt_token_supply
        .checked_add(amount_of_debt_token)
        .ok_or(Error::MathOverflowError)?;
    let amount_to_sub = amount.checked_neg().ok_or(Error::MathOverflowError)?;

    add_token_balance(env, &reserve.debt_token_address, who, amount_of_debt_token)?;
    add_stoken_underlying_balance(env, &reserve.s_token_address, amount_to_sub)?;
    write_token_total_supply(env, &reserve.debt_token_address, debt_token_supply_after)?;

    user_configurator
        .borrow(reserve.get_id(), debt_balance == 0)?
        .write();

    event::borrow(env, who, asset, amount);

    recalculate_reserve_data(
        env,
        asset,
        &reserve,
        s_token_supply,
        debt_token_supply_after,
    )?;

    Ok(vec![
        env,
        MintBurn::new(
            AssetBalance::new(reserve.debt_token_address, amount_of_debt_token),
            true,
            who.clone(),
        ),
        MintBurn::new(AssetBalance::new(asset.clone(), amount), true, who.clone()),
        MintBurn::new(
            AssetBalance::new(asset.clone(), amount),
            false,
            reserve.s_token_address,
        ),
    ])
}
