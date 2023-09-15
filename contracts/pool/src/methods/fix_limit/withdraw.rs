use pool_interface::types::asset_balance::AssetBalance;
use pool_interface::types::error::Error;
use pool_interface::types::mint_burn::MintBurn;
use soroban_sdk::{assert_with_error, vec, Address, Env, Vec};

use crate::event;
use crate::methods::fix_limit::account_position::calc_account_data;
use crate::methods::utils::get_collat_coeff::get_collat_coeff;
use crate::methods::utils::recalculate_reserve_data::recalculate_reserve_data;
use crate::methods::utils::validation::{
    require_active_reserve, require_good_position, require_not_paused, require_positive_amount,
};
use crate::storage::{
    add_stoken_underlying_balance, add_token_balance, read_reserve, read_token_balance,
    read_token_total_supply, write_token_total_supply,
};
use crate::types::calc_account_data_cache::CalcAccountDataCache;
use crate::types::user_configurator::UserConfigurator;

pub fn withdraw(
    env: &Env,
    who: &Address,
    asset: &Address,
    amount: i128,
    to: &Address,
) -> Result<Vec<MintBurn>, Error> {
    who.require_auth();

    require_not_paused(env);
    require_positive_amount(env, amount);

    let reserve = read_reserve(env, asset)?;
    require_active_reserve(env, &reserve);

    let debt_token_supply = read_token_total_supply(env, &reserve.debt_token_address);
    let s_token_supply = read_token_total_supply(env, &reserve.s_token_address);

    let collat_coeff = get_collat_coeff(env, &reserve, s_token_supply, debt_token_supply)?;

    let collat_balance = read_token_balance(env, &reserve.s_token_address, who);
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
            &CalcAccountDataCache {
                mb_who_collat: Some(&AssetBalance::new(
                    reserve.s_token_address.clone(),
                    collat_balance_after,
                )),
                mb_who_debt: None,
                mb_s_token_supply: Some(&AssetBalance::new(
                    reserve.s_token_address.clone(),
                    s_token_supply_after,
                )),
                mb_debt_token_supply: Some(&AssetBalance::new(
                    reserve.debt_token_address.clone(),
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

    let s_token_to_sub = s_token_to_burn
        .checked_neg()
        .ok_or(Error::MathOverflowError)?;

    // s_token.burn(who, &s_token_to_burn, &underlying_to_withdraw, to);
    let mint_burn_1 = MintBurn {
        asset_balance: AssetBalance {
            asset: reserve.s_token_address.clone(),
            balance: s_token_to_burn,
        },
        mint: false,
        who: who.clone(),
    };
    let mint_burn_2 = MintBurn {
        asset_balance: AssetBalance {
            asset: asset.clone(),
            balance: underlying_to_withdraw,
        },
        mint: true,
        who: to.clone(),
    };
    let mint_burn_3 = MintBurn {
        asset_balance: AssetBalance {
            asset: asset.clone(),
            balance: underlying_to_withdraw,
        },
        mint: false,
        who: reserve.s_token_address.clone(),
    };

    add_token_balance(env, &reserve.s_token_address, who, s_token_to_sub)?;
    write_token_total_supply(env, &reserve.s_token_address, s_token_supply_after)?;
    add_stoken_underlying_balance(env, &reserve.s_token_address, amount_to_sub)?;

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

    Ok(vec![env, mint_burn_1, mint_burn_2, mint_burn_3])
}
