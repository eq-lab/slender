use common::FixedI128;
use flash_loan_receiver_interface::{FlashLoanReceiverClient, LoanAsset as ReceiverAsset};
use pool_interface::types::error::Error;
use pool_interface::types::flash_loan_asset::FlashLoanAsset;
use s_token_interface::STokenClient;
use soroban_sdk::{assert_with_error, token, vec, Address, Bytes, Env, Vec};

use crate::methods::utils::get_fungible_lp_tokens::get_fungible_lp_tokens;
use crate::methods::utils::validation::require_not_in_grace_period;
use crate::storage::{read_reserve, read_token_balance, read_token_total_supply};
use crate::{add_protocol_fee_vault, event, read_pause_info, read_pool_config};

use super::borrow::do_borrow;
use super::utils::recalculate_reserve_data::recalculate_reserve_data;
use super::utils::validation::{
    require_active_reserve, require_borrowing_enabled, require_not_paused, require_positive_amount,
};

pub fn flash_loan(
    env: &Env,
    who: &Address,
    receiver: &Address,
    loan_assets: &Vec<FlashLoanAsset>,
    params: &Bytes,
) -> Result<(), Error> {
    who.require_auth();
    let pause_info = read_pause_info(env)?;
    require_not_paused(env, &pause_info);

    let pool_config = read_pool_config(env)?;
    let fee =
        FixedI128::from_percentage(pool_config.flash_loan_fee).ok_or(Error::MathOverflowError)?;

    let loan_asset_len = loan_assets.len();
    assert_with_error!(env, loan_asset_len > 0, Error::BellowMinValue);

    let mut receiver_assets = vec![env];
    let mut reserves = vec![env];

    for i in 0..loan_asset_len {
        let loan_asset = loan_assets.get_unchecked(i);

        require_positive_amount(env, loan_asset.amount);

        let reserve = read_reserve(env, &loan_asset.asset)?;
        require_active_reserve(env, &reserve);
        require_borrowing_enabled(env, &reserve);

        if loan_asset.borrow {
            require_not_in_grace_period(env, &pause_info);
        }

        let (s_token_address, _) = get_fungible_lp_tokens(&reserve)?;

        let s_token = STokenClient::new(env, s_token_address);
        s_token.transfer_underlying_to(receiver, &loan_asset.amount);

        reserves.push_back(reserve);
        receiver_assets.push_back(ReceiverAsset {
            asset: loan_asset.asset,
            amount: loan_asset.amount,
            premium: fee
                .mul_int(loan_asset.amount)
                .ok_or(Error::MathOverflowError)?,
            borrow: loan_asset.borrow,
        });
    }

    let loan_receiver = FlashLoanReceiverClient::new(env, receiver);
    let loan_received = loan_receiver.receive(who, &receiver_assets, params);
    assert_with_error!(env, loan_received, Error::FlashLoanReceiverError);

    for i in 0..loan_asset_len {
        let loan_asset = loan_assets.get_unchecked(i);
        let received_asset = receiver_assets.get_unchecked(i);
        let reserve = reserves.get_unchecked(i);
        let (s_token_address, debt_token_address) = get_fungible_lp_tokens(&reserve)?;
        if !loan_asset.borrow {
            let underlying_asset = token::Client::new(env, &received_asset.asset);

            underlying_asset.transfer_from(
                &env.current_contract_address(),
                receiver,
                s_token_address,
                &received_asset.amount,
            );

            add_protocol_fee_vault(env, &received_asset.asset, received_asset.premium)?;
        } else {
            let s_token_supply = read_token_total_supply(env, s_token_address);

            let debt_token_supply_after = do_borrow(
                env,
                who,
                &received_asset.asset,
                &reserve,
                &pool_config,
                read_token_balance(env, s_token_address, who),
                read_token_balance(env, debt_token_address, who),
                s_token_supply,
                read_token_total_supply(env, debt_token_address),
                received_asset.amount,
                s_token_address,
                debt_token_address,
            )?;

            recalculate_reserve_data(
                env,
                &received_asset.asset,
                &reserve,
                &pool_config,
                s_token_supply,
                debt_token_supply_after,
            )?;
        }

        let premium = if loan_asset.borrow {
            0
        } else {
            received_asset.premium
        };

        event::flash_loan(
            env,
            who,
            receiver,
            &received_asset.asset,
            received_asset.amount,
            premium,
            loan_asset.borrow,
        );
    }

    Ok(())
}
