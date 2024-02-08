use pool_interface::types::reserve_type::ReserveType;
use pool_interface::types::{error::Error, reserve_data::ReserveData};
use s_token_interface::STokenClient;
use soroban_sdk::{token, Address, Env};

use crate::event;
use crate::storage::{
    add_stoken_underlying_balance, read_reserve, read_stoken_underlying_balance,
    read_token_balance, read_token_total_supply, write_token_balance, write_token_total_supply,
};
use crate::types::user_configurator::UserConfigurator;

use super::utils::get_collat_coeff::get_collat_coeff;
use super::utils::recalculate_reserve_data::recalculate_reserve_data;
use super::utils::validation::{
    require_active_reserve, require_liquidity_cap_not_exceeded, require_not_paused,
    require_positive_amount, require_zero_debt,
};

pub fn deposit(env: &Env, who: &Address, asset: &Address, amount: i128) -> Result<(), Error> {
    who.require_auth();

    require_not_paused(env);
    require_positive_amount(env, amount);

    let reserve = read_reserve(env, asset)?;
    require_active_reserve(env, &reserve);

    let mut user_configurator = UserConfigurator::new(env, who, true);
    let user_config = user_configurator.user_config()?;
    require_zero_debt(env, user_config, reserve.get_id());

    let is_first_deposit =
        if let ReserveType::Fungible(s_token_address, debt_token_address) = &reserve.reserve_type {
            let debt_token_supply = read_token_total_supply(env, debt_token_address);

            let (is_first_deposit, s_token_supply_after) = do_deposit_fungible(
                env,
                who,
                asset,
                &reserve,
                read_token_total_supply(env, s_token_address),
                debt_token_supply,
                read_token_balance(env, s_token_address, who),
                amount,
                s_token_address,
            )?;

            recalculate_reserve_data(
                env,
                asset,
                &reserve,
                s_token_supply_after,
                debt_token_supply,
            )?;

            is_first_deposit
        } else {
            do_deposit_rwa(env, who, asset, amount)?
        };

    event::deposit(env, who, asset, amount);

    user_configurator
        .deposit(reserve.get_id(), asset, is_first_deposit)?
        .write();

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn do_deposit_fungible(
    env: &Env,
    who: &Address,
    asset: &Address,
    reserve: &ReserveData,
    s_token_supply: i128,
    debt_token_supply: i128,
    who_collat: i128,
    amount: i128,
    s_token_address: &Address,
) -> Result<(bool, i128), Error> {
    let balance = read_stoken_underlying_balance(env, s_token_address);
    require_liquidity_cap_not_exceeded(env, reserve, debt_token_supply, balance, amount)?;

    let collat_coeff = get_collat_coeff(
        env,
        reserve,
        s_token_supply,
        read_stoken_underlying_balance(env, s_token_address),
        debt_token_supply,
    )?;
    let is_first_deposit = who_collat == 0;
    let amount_to_mint = collat_coeff
        .recip_mul_int(amount)
        .ok_or(Error::MathOverflowError)?;
    let s_token_supply_after = s_token_supply
        .checked_add(amount_to_mint)
        .ok_or(Error::MathOverflowError)?;
    let who_collat_after = who_collat
        .checked_add(amount_to_mint)
        .ok_or(Error::MathOverflowError)?;

    token::Client::new(env, asset).transfer(who, s_token_address, &amount);
    STokenClient::new(env, s_token_address).mint(who, &amount_to_mint);

    add_stoken_underlying_balance(env, s_token_address, amount)?;
    write_token_total_supply(env, s_token_address, s_token_supply_after)?;
    write_token_balance(env, s_token_address, who, who_collat_after)?;

    Ok((is_first_deposit, s_token_supply_after))
}

fn do_deposit_rwa(env: &Env, who: &Address, asset: &Address, amount: i128) -> Result<bool, Error> {
    let balance_before = read_token_balance(env, asset, who);
    token::Client::new(env, asset).transfer(who, &env.current_contract_address(), &amount);
    let balance_after = balance_before
        .checked_add(amount)
        .ok_or(Error::MathOverflowError)?;
    write_token_balance(env, asset, who, balance_after)?;

    Ok(balance_before == 0)
}
