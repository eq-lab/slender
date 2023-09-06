use common::FixedI128;
use debt_token_interface::DebtTokenClient;
use flash_loan_receiver_interface::{Asset as ReceiverAsset, FlashLoanReceiverClient};
use pool_interface::types::error::Error;
use pool_interface::types::flash_loan_asset::FlashLoanAsset;
use s_token_interface::STokenClient;
use soroban_sdk::{assert_with_error, token, vec, Address, Bytes, Env, Vec};

use crate::event;
use crate::methods::borrow::do_borrow;
use crate::methods::init_reserve::recalculate_reserve_data;
use crate::methods::validation::{
    require_active_reserve, require_borrowing_enabled, require_not_paused, require_positive_amount,
};
use crate::storage::{read_flash_loan_fee, read_reserve, read_treasury};

#[cfg(not(feature = "exceeded-limit-fix"))]
pub fn flash_loan(
    env: &Env,
    who: &Address,
    receiver: &Address,
    loan_assets: &Vec<FlashLoanAsset>,
    params: &Bytes,
) -> Result<(), Error> {
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

        let s_token = STokenClient::new(env, &reserve.s_token_address);
        s_token.transfer_underlying_to(receiver, &loan_asset.amount);

        reserves.push_back(reserve);
        receiver_assets.push_back(ReceiverAsset {
            asset: loan_asset.asset,
            amount: loan_asset.amount,
            premium: fee
                .mul_int(loan_asset.amount)
                .ok_or(Error::MathOverflowError)?,
        });
    }

    let loan_receiver = FlashLoanReceiverClient::new(env, receiver);
    let loan_received = loan_receiver.receive(&receiver_assets, params);
    assert_with_error!(env, loan_received, Error::FlashLoanReceiverError);

    let treasury = read_treasury(env);

    for i in 0..loan_asset_len {
        let loan_asset = loan_assets.get_unchecked(i);
        let received_asset = receiver_assets.get_unchecked(i);
        let reserve = reserves.get_unchecked(i);

        if !loan_asset.borrow {
            let amount_with_premium = received_asset
                .amount
                .checked_add(received_asset.premium)
                .ok_or(Error::MathOverflowError)?;

            let underlying_asset = token::Client::new(env, &received_asset.asset);
            let s_token = STokenClient::new(env, &reserve.s_token_address);

            underlying_asset.transfer_from(
                &env.current_contract_address(),
                receiver,
                &reserve.s_token_address,
                &amount_with_premium,
            );
            s_token.transfer_underlying_to(&treasury, &received_asset.premium);
        } else {
            let s_token = STokenClient::new(env, &reserve.s_token_address);
            let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
            let s_token_supply = s_token.total_supply();

            let debt_token_supply_after = do_borrow(
                env,
                who,
                &received_asset.asset,
                &reserve,
                s_token.balance(who),
                debt_token.balance(who),
                s_token_supply,
                debt_token.total_supply(),
                received_asset.amount,
            )?;

            recalculate_reserve_data(
                env,
                &received_asset.asset,
                &reserve,
                s_token_supply,
                debt_token_supply_after,
            )?;
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

    Ok(())
}

#[cfg(feature = "exceeded-limit-fix")]
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

            let asset_price =
                get_asset_price(env, &loan_asset.asset, reserve.configuration.is_base_asset)?;
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
                None,
                Some(&AssetBalance::new(
                    reserve.debt_token_address.clone(),
                    debt_balance,
                )),
                None,
                None,
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

            add_token_balance(
                env,
                &reserve.debt_token_address,
                who,
                amount_of_debt_token,
            )?;
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
