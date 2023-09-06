use debt_token_interface::DebtTokenClient;
use pool_interface::types::{error::Error, reserve_data::ReserveData};
use s_token_interface::STokenClient;
use soroban_sdk::{token, Address, Env};

use crate::event;
use crate::storage::{add_stoken_underlying_balance, read_reserve, read_stoken_underlying_balance};
use crate::types::user_configurator::UserConfigurator;

use super::collat_coeff::get_collat_coeff;
use super::init_reserve::recalculate_reserve_data;
use super::validation::{
    require_active_reserve, require_liq_cap_not_exceeded, require_not_paused,
    require_positive_amount, require_zero_debt,
};

#[cfg(not(feature = "exceeded-limit-fix"))]
pub fn deposit(env: &Env, who: &Address, asset: &Address, amount: i128) -> Result<(), Error> {
    who.require_auth();

    require_not_paused(env);
    require_positive_amount(env, amount);

    let reserve = read_reserve(env, asset)?;
    require_active_reserve(env, &reserve);

    let mut user_configurator = UserConfigurator::new(env, who, true);
    let user_config = user_configurator.user_config()?;
    require_zero_debt(env, user_config, reserve.get_id());

    let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
    let s_token = STokenClient::new(env, &reserve.s_token_address);
    let debt_token_supply = debt_token.total_supply();

    let (is_first_deposit, s_token_supply_after) = do_deposit(
        env,
        who,
        asset,
        &reserve,
        s_token.total_supply(),
        debt_token_supply,
        s_token.balance(who),
        amount,
    )?;

    user_configurator
        .deposit(reserve.get_id(), asset, is_first_deposit)?
        .write();

    recalculate_reserve_data(
        env,
        asset,
        &reserve,
        s_token_supply_after,
        debt_token_supply,
    )?;

    Ok(())
}

#[cfg(feature = "exceeded-limit-fix")]
pub fn deposit(
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

    let mut user_configurator = UserConfigurator::new(env, who, true);
    let user_config = user_configurator.user_config()?;
    require_zero_debt(env, user_config, reserve.get_id());

    let debt_token_supply = read_token_total_supply(env, &reserve.debt_token_address);
    let s_token_supply = read_token_total_supply(env, &reserve.s_token_address);

    let balance = read_stoken_underlying_balance(env, &reserve.s_token_address);
    require_liq_cap_not_exceeded(env, &reserve, debt_token_supply, balance, amount)?;

    let collat_coeff = get_collat_coeff(env, &reserve, s_token_supply, debt_token_supply)?;
    let amount_to_mint = collat_coeff
        .recip_mul_int(amount)
        .ok_or(Error::MathOverflowError)?;
    let s_token_supply_after = s_token_supply
        .checked_add(amount_to_mint)
        .ok_or(Error::MathOverflowError)?;
    let is_first_deposit = read_token_balance(env, &reserve.s_token_address, who) == 0i128;

    // token::Client::new(env, asset).transfer(who, &reserve.s_token_address, &amount);
    let mint_burn_1 = MintBurn {
        asset_balance: AssetBalance {
            asset: asset.clone(),
            balance: amount,
        },
        mint: false,
        who: who.clone(),
    };
    let mint_burn_2 = MintBurn {
        asset_balance: AssetBalance {
            asset: asset.clone(),
            balance: amount,
        },
        mint: true,
        who: reserve.s_token_address.clone(),
    };
    add_stoken_underlying_balance(env, &reserve.s_token_address, amount)?;

    // STokenClient::new(env, &reserve.s_token_address).mint(who, &amount_to_mint);
    let mint_burn_3 = MintBurn {
        asset_balance: AssetBalance {
            asset: reserve.s_token_address.clone(),
            balance: amount_to_mint,
        },
        mint: true,
        who: who.clone(),
    };
    add_token_balance(env, &reserve.s_token_address, who, amount_to_mint)?;
    add_token_total_supply(env, &reserve.s_token_address, amount_to_mint)?;

    user_configurator
        .deposit(reserve.get_id(), asset, is_first_deposit)?
        .write();

    recalculate_reserve_data(
        env,
        asset,
        &reserve,
        s_token_supply_after,
        debt_token_supply,
    )?;

    event::deposit(env, who, asset, amount);

    Ok(vec![env, mint_burn_1, mint_burn_2, mint_burn_3])
}

#[allow(clippy::too_many_arguments)]
#[cfg(not(feature = "exceeded-limit-fix"))]
fn do_deposit(
    env: &Env,
    who: &Address,
    asset: &Address,
    reserve: &ReserveData,
    s_token_supply: i128,
    debt_token_supply: i128,
    who_collat: i128,
    amount: i128,
) -> Result<(bool, i128), Error> {
    let balance = read_stoken_underlying_balance(env, &reserve.s_token_address);
    require_liq_cap_not_exceeded(env, reserve, debt_token_supply, balance, amount)?;

    let collat_coeff = get_collat_coeff(env, reserve, s_token_supply, debt_token_supply)?;
    let amount_to_mint = collat_coeff
        .recip_mul_int(amount)
        .ok_or(Error::MathOverflowError)?;
    let s_token_supply_after = s_token_supply
        .checked_add(amount_to_mint)
        .ok_or(Error::MathOverflowError)?;

    let is_first_deposit = who_collat == 0;

    token::Client::new(env, asset).transfer(who, &reserve.s_token_address, &amount);
    add_stoken_underlying_balance(env, &reserve.s_token_address, amount)?;
    STokenClient::new(env, &reserve.s_token_address).mint(who, &amount_to_mint);

    event::deposit(env, who, asset, amount);

    Ok((is_first_deposit, s_token_supply_after))
}
