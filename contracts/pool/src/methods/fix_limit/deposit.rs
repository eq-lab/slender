use pool_interface::types::asset_balance::AssetBalance;
use pool_interface::types::error::Error;
use pool_interface::types::mint_burn::MintBurn;
use soroban_sdk::{vec, Address, Env, Vec};

use crate::event;
use crate::methods::utils::get_collat_coeff::get_collat_coeff;
use crate::methods::utils::recalculate_reserve_data::recalculate_reserve_data;
use crate::methods::utils::validation::{
    require_active_reserve, require_liq_cap_not_exceeded, require_not_paused,
    require_positive_amount, require_zero_debt,
};
use crate::storage::{
    add_stoken_underlying_balance, add_token_balance, read_reserve, read_stoken_underlying_balance,
    read_token_balance, read_token_total_supply, write_token_total_supply,
};
use crate::types::user_configurator::UserConfigurator;

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
    write_token_total_supply(env, &reserve.s_token_address, s_token_supply_after)?;

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
