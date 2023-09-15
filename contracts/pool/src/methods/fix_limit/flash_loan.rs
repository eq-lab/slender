use common::FixedI128;
use flash_loan_receiver_interface::Asset as ReceiverAsset;
use pool_interface::types::asset_balance::AssetBalance;
use pool_interface::types::error::Error;
use pool_interface::types::flash_loan_asset::FlashLoanAsset;
use pool_interface::types::mint_burn::MintBurn;
use soroban_sdk::{assert_with_error, vec, Address, Bytes, Env, Vec};

use crate::event;
use crate::methods::fix_limit::account_position::{calc_account_data, CalcAccountDataCache};
use crate::methods::utils::rate::get_actual_borrower_accrued_rate;
use crate::methods::utils::recalculate_reserve_data::recalculate_reserve_data;
use crate::methods::utils::validation::{
    require_active_reserve, require_borrowing_enabled, require_not_in_collateral_asset,
    require_not_paused, require_positive_amount, require_util_cap_not_exceeded,
};
use crate::storage::{
    add_stoken_underlying_balance, add_token_balance, add_token_total_supply, read_flash_loan_fee,
    read_price, read_reserve, read_token_balance, read_token_total_supply, read_treasury,
};
use crate::types::user_configurator::UserConfigurator;

pub fn flash_loan(
    env: &Env,
    who: &Address,
    receiver: &Address,
    loan_assets: &Vec<FlashLoanAsset>,
    _params: &Bytes,
) -> Result<Vec<MintBurn>, Error> {
    who.require_auth();
    require_not_paused(env);

    let fee =
        FixedI128::from_percentage(read_flash_loan_fee(env)).ok_or(Error::MathOverflowError)?;

    let loan_asset_len = loan_assets.len();
    assert_with_error!(env, loan_asset_len > 0, Error::MustBePositive);

    let mut receiver_assets = vec![env];
    let mut reserves = vec![env];

    for i in 0..loan_asset_len {
        let loan_asset = loan_assets.get_unchecked(i);

        require_positive_amount(env, loan_asset.amount);

        let reserve = read_reserve(env, &loan_asset.asset)?;
        require_active_reserve(env, &reserve);
        require_borrowing_enabled(env, &reserve);

        reserves.push_back(reserve);
        receiver_assets.push_back(ReceiverAsset {
            asset: loan_asset.asset,
            amount: loan_asset.amount,
            premium: fee
                .mul_int(loan_asset.amount)
                .ok_or(Error::MathOverflowError)?,
        });
    }

    let treasury = read_treasury(env);
    let mut mints_burns = vec![env];

    for i in 0..loan_asset_len {
        let loan_asset = loan_assets.get_unchecked(i);
        let received_asset = receiver_assets.get_unchecked(i);
        let reserve = reserves.get_unchecked(i);

        if !loan_asset.borrow {
            mints_burns.push_back(MintBurn::new(
                AssetBalance::new(received_asset.asset.clone(), received_asset.premium),
                true,
                treasury.clone(),
            ));
        } else {
            let collat_balance = read_token_balance(env, &reserve.s_token_address, who);

            require_not_in_collateral_asset(env, collat_balance);

            let s_token_supply = read_token_total_supply(env, &reserve.s_token_address);
            let debt_token_supply = read_token_total_supply(env, &reserve.debt_token_address);

            let asset_price = FixedI128::from_inner(read_price(env, &loan_asset.asset));
            let amount_in_xlm = asset_price
                .mul_int(loan_asset.amount)
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
                .recip_mul_int(loan_asset.amount)
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
            let amount_to_sub = loan_asset
                .amount
                .checked_neg()
                .ok_or(Error::MathOverflowError)?;

            add_token_balance(env, &reserve.debt_token_address, who, amount_of_debt_token)?;
            add_stoken_underlying_balance(env, &reserve.s_token_address, amount_to_sub)?;
            add_token_total_supply(env, &reserve.debt_token_address, amount_of_debt_token)?;

            user_configurator
                .borrow(reserve.get_id(), debt_balance == 0)?
                .write();

            event::borrow(env, who, &loan_asset.asset, loan_asset.amount);

            recalculate_reserve_data(
                env,
                &received_asset.asset,
                &reserve,
                s_token_supply,
                debt_token_supply_after,
            )?;

            mints_burns.push_back(MintBurn::new(
                AssetBalance::new(reserve.debt_token_address.clone(), amount_of_debt_token),
                true,
                who.clone(),
            ));
            mints_burns.push_back(MintBurn::new(
                AssetBalance::new(loan_asset.asset.clone(), loan_asset.amount),
                true,
                who.clone(),
            ));
            mints_burns.push_back(MintBurn::new(
                AssetBalance::new(loan_asset.asset.clone(), loan_asset.amount),
                false,
                reserve.s_token_address,
            ));
        }

        event::flash_loan(
            env,
            who,
            receiver,
            &received_asset.asset,
            received_asset.amount,
            received_asset.premium,
        );
    }

    Ok(mints_burns)
}
