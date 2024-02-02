use common::FixedI128;
use flash_loan_receiver_interface::{FlashLoanReceiverClient, LoanAsset as ReceiverAsset};
use pool_interface::types::error::Error;
use pool_interface::types::flash_loan_asset::FlashLoanAsset;
use s_token_interface::STokenClient;
use soroban_sdk::{assert_with_error, token, vec, Address, Bytes, Env, Vec};

use crate::event;
use crate::methods::utils::get_fungible_lp_tokens::get_fungible_lp_tokens;
use crate::storage::{
    read_flash_loan_fee, read_reserve, read_token_balance, read_token_total_supply, read_treasury,
};

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

    let treasury = read_treasury(env);

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
            underlying_asset.transfer_from(
                &env.current_contract_address(),
                receiver,
                &treasury,
                &received_asset.premium,
            );
        } else {
            let s_token_supply = read_token_total_supply(env, s_token_address);

            let debt_token_supply_after = do_borrow(
                env,
                who,
                &received_asset.asset,
                &reserve,
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
