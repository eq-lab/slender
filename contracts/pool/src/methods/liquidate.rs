use common::{FixedI128, PERCENTAGE_FACTOR};
use debt_token_interface::DebtTokenClient;
use pool_interface::types::{error::Error, reserve_type::ReserveType};
use s_token_interface::STokenClient;
use soroban_sdk::{assert_with_error, token, Address, Env};

use crate::methods::utils::recalculate_reserve_data::recalculate_reserve_data;
use crate::types::account_data::AccountData;
use crate::types::calc_account_data_cache::CalcAccountDataCache;
use crate::types::liquidation_asset::LiquidationAsset;
use crate::types::price_provider::PriceProvider;
use crate::types::user_configurator::UserConfigurator;
use crate::{
    add_stoken_underlying_balance, event, read_initial_health, read_token_balance,
    read_token_total_supply, write_token_balance, write_token_total_supply,
};  

use super::account_position::calc_account_data;
use super::utils::validation::require_not_paused;
//@audit there does not seem to be an incentive to liquidate small positions... and we can open them/repay to create them easily. 
pub fn liquidate( 
    env: &Env,
    liquidator: &Address,
    who: &Address,
    receive_stoken: bool,
) -> Result<(), Error> {
    liquidator.require_auth();

    require_not_paused(env); //@audit  1 read

    let mut user_configurator = UserConfigurator::new(env, who, false);
    let user_config = user_configurator.user_config()?; //@audit 1 read
    let mut price_provider = PriceProvider::new(env)?;

    let account_data = calc_account_data(
        env,
        who,
        &CalcAccountDataCache::none(),
        user_config,
        &mut price_provider,
        true,
    )?; // [5+CLIENT_PRICES_READS (liquidation) reads] times the number of assets used as collateral or for borrowing

    assert_with_error!(env, !account_data.is_good_position(), Error::GoodPosition);
    //@audit from start up to this point, 
    let (debt_covered_in_base, total_liq_in_base) = do_liquidate(
        env,
        liquidator,
        who,
        account_data,
        &mut user_configurator,
        receive_stoken,
        &mut price_provider,
    )?;

    event::liquidation(env, who, debt_covered_in_base, total_liq_in_base);

    Ok(())
}

fn do_liquidate(
    env: &Env,
    liquidator: &Address,
    who: &Address,
    account_data: AccountData,
    user_configurator: &mut UserConfigurator,
    receive_stoken: bool,
    price_provider: &mut PriceProvider,
) -> Result<(i128, i128), Error> {
    let mut total_debt_after_in_base = account_data.debt;
    let mut total_collat_disc_after_in_base = account_data.discounted_collateral;
    let mut total_debt_to_cover_in_base = 0i128;
    let mut total_liq_in_base = 0i128;

    let zero_percent = FixedI128::from_inner(0);
    let initial_health_percent = FixedI128::from_percentage(read_initial_health(env)?).unwrap(); //@audit 1 read
    let hundred_percent = FixedI128::from_percentage(PERCENTAGE_FACTOR).unwrap();
    let npv_percent = FixedI128::from_rational(account_data.npv, total_collat_disc_after_in_base)
        .ok_or(Error::LiquidateMathError)?; //@audit shouldn't this be converted to percentage? 
    
    //@audit we are ROUNDING DOWN here
    //@audit is this correct? Why is it "from_rational" and not converted to percentage? (though it could theoretically cross the 100% here)
    let liq_bonus_percent = npv_percent.min(zero_percent).abs().min(hundred_percent);
    //@audit is there an incentive to liquidate positions that are SERIOUSLY in debt, i.e., above 100%? 
    // I think yes because total_debt_liq_bonus_percent can be hundred_percent ... then you are basically getting the collateral for free.
    // question is why though.
    let total_debt_liq_bonus_percent = hundred_percent
        .checked_sub(liq_bonus_percent)
        .ok_or(Error::LiquidateMathError)?;

    let safe_collat_percent = hundred_percent.checked_sub(initial_health_percent).unwrap(); //@audit 100% - (initial_health)%
    //@audit shouldn't the liquidation order be proportional to collateral discount? 
    for collat in account_data.liq_collats.ok_or(Error::LiquidateMathError)? {
        let discount_percent =
            FixedI128::from_percentage(collat.reserve.configuration.discount).unwrap();

        // the same for token-based RWA
        let liq_comp_amount = calc_liq_amount(
            price_provider,
            &collat,
            hundred_percent,
            discount_percent,
            liq_bonus_percent,
            safe_collat_percent,
            initial_health_percent,
            total_collat_disc_after_in_base,
            total_debt_after_in_base,
        )?; //@audit takes 0, CLIENT_PRICES_READS, or 1+CLIENT_PRICES_READS reads

        let total_sub_comp_amount = discount_percent
            .mul_int(liq_comp_amount)
            .ok_or(Error::LiquidateMathError)?; //@audit division followed by multiplication - but doesn't seem bad since we are looking at a fixed percentage set by the admin and not an on-chain computed amount

        let total_sub_amount_in_base =
            price_provider.convert_to_base(&collat.asset, total_sub_comp_amount)?; //@audit if the oracle is down, can't liquidate
        //@audit takes 0, CLIENT_PRICES_READS, or 1+CLIENT_PRICES_READS reads
        let debt_comp_amount = total_debt_liq_bonus_percent
            .mul_int(liq_comp_amount)
            .ok_or(Error::LiquidateMathError)?;

        let debt_in_base = price_provider.convert_to_base(&collat.asset, debt_comp_amount)?;
        //@audit takes 0, CLIENT_PRICES_READS, or 1+CLIENT_PRICES_READS reads
        total_debt_after_in_base = total_debt_after_in_base
            .checked_sub(debt_in_base)
            .ok_or(Error::LiquidateMathError)?;

        total_collat_disc_after_in_base = total_collat_disc_after_in_base
            .checked_sub(total_sub_amount_in_base)
            .ok_or(Error::LiquidateMathError)?;

        total_liq_in_base = total_liq_in_base
            .checked_add(price_provider.convert_to_base(&collat.asset, liq_comp_amount)?)
            .ok_or(Error::LiquidateMathError)?;
        //@audit takes 0, CLIENT_PRICES_READS, or 1+CLIENT_PRICES_READS reads
        if let ReserveType::Fungible(s_token_address, debt_token_address) =
            &collat.reserve.reserve_type
        {
            let mut s_token_supply = read_token_total_supply(env, s_token_address); //@audit 1 read
            let debt_token_supply = read_token_total_supply(env, debt_token_address); //@audit 1 read

            let liq_lp_amount = FixedI128::from_inner(collat.coeff.unwrap())
                .recip_mul_int(liq_comp_amount)
                .ok_or(Error::LiquidateMathError)?;

            let s_token = STokenClient::new(env, s_token_address);

            if receive_stoken {
                let mut liquidator_configurator = UserConfigurator::new(env, liquidator, true);
                let liquidator_config = liquidator_configurator.user_config()?; //@audit 1 read

                assert_with_error!(
                    env,
                    !liquidator_config.is_borrowing(env, collat.reserve.get_id()),
                    Error::MustNotHaveDebt
                ); //@audit note: it is possible to use the reserve as collateral though - so self liquidation seems still possible...

                let liquidator_collat_before =
                    read_token_balance(env, &s_token.address, liquidator); //@audit 1 read

                let liquidator_collat_after = liquidator_collat_before
                    .checked_add(liq_lp_amount)
                    .ok_or(Error::LiquidateMathError)?;

                s_token.transfer_on_liquidation(who, liquidator, &liq_lp_amount); //@audit 7 read + 2 write
                write_token_balance(env, &s_token.address, liquidator, liquidator_collat_after)?; //@audit 1 write
                //@audit if who == liquidator, we essentially just increased its amount of tokens by liq_lp_amount? 
                let use_as_collat = liquidator_collat_before == 0;
                 //@audit shouldn't we write .get_id(), &collat.asset, use_as_collat)?
                liquidator_configurator
                    .deposit(collat.reserve.get_id(), &collat.asset, use_as_collat)?
                    .write(); //@audit 1 read + 1 write
            } else {
                let amount_to_sub = liq_lp_amount
                    .checked_neg()
                    .ok_or(Error::LiquidateMathError)?;
                s_token_supply = s_token_supply
                    .checked_sub(liq_lp_amount)
                    .ok_or(Error::LiquidateMathError)?; //@audit updated s_token_supply
                //@audit liq_lp_amount >0 as to not revert here
                s_token.burn(who, &liq_lp_amount, &liq_comp_amount, liquidator); //@audit 4 read + 1 write
                add_stoken_underlying_balance(env, &s_token.address, amount_to_sub)?; //@audit 1 read + 1 write
                //@audit note: if who != liquidator, the liquidator s_token balance has not change - he only got underlying tokens. 
            }
            //@audit this is correct, but since we have a bug in Fungible tokens when who == liquidator, this quantity would be wrong. 
            //... does this lead to something interesting? 
            write_token_total_supply(env, s_token_address, s_token_supply)?; //@audit 1 write
            write_token_balance(
                env,
                &s_token.address,
                who,
                collat.lp_balance.unwrap() - liq_lp_amount,
            )?; //@audit 1 write
            //@audit here we are fixing the bug we had before! quantity is correct now. 
            //@audit we are only updating lp_balance - not comp_balance. Is that important? 
            recalculate_reserve_data(
                env,
                &collat.asset,
                &collat.reserve,
                s_token_supply,
                debt_token_supply,
            )?; //@audit no reads or writes
        } else {
            let who_rwa_balance_before = read_token_balance(env, &collat.asset, who); //@audit 1 read
            let who_rwa_balance_after = who_rwa_balance_before
                .checked_sub(liq_comp_amount)
                .ok_or(Error::MathOverflowError)?;
            token::Client::new(env, &collat.asset).transfer(
                &env.current_contract_address(),
                liquidator,
                &liq_comp_amount,
            );
            write_token_balance(env, &collat.asset, who, who_rwa_balance_after)?; //@audit 1 write
            //@audit would this would be wrong if who == liquidator? 
            // It actually seems right since we sent liq_comp_amount to the liquidator and then wrote the correct new balance in the contract...
            // TODO: does this create a problem somehow? 
        }

        user_configurator.withdraw(
            collat.reserve.get_id(),
            &collat.asset,
            collat.comp_balance == liq_comp_amount,
        )?; //@audit 1 read

        total_debt_to_cover_in_base += debt_in_base;

        let npv_after = total_collat_disc_after_in_base
            .checked_sub(total_debt_after_in_base)
            .ok_or(Error::LiquidateMathError)?;

        if npv_after.is_positive() {
            break;
        }
    }
    //@audit we can liquidate a user even if npv_after is not only negative but actually worse then the original npv!
    let debt_covered_in_base = total_debt_to_cover_in_base;

    for debt in account_data.liq_debts.ok_or(Error::LiquidateMathError)? {
        if total_debt_to_cover_in_base.eq(&0) { 
            break;
        }

        if let ReserveType::Fungible(s_token_address, debt_token_address) =
            &debt.reserve.reserve_type
        {
            let debt_comp_in_base =
                price_provider.convert_to_base(&debt.asset, debt.comp_balance)?; //@audit takes 0, CLIENT_PRICES_READS, or 1+CLIENT_PRICES_READS reads

            let (debt_lp_to_burn, debt_comp_to_transfer) =
                if total_debt_to_cover_in_base >= debt_comp_in_base {
                    total_debt_to_cover_in_base -= debt_comp_in_base;

                    user_configurator.repay(debt.reserve.get_id(), true)?; //@audit 1 read

                    (debt.lp_balance.unwrap(), debt.comp_balance)
                } else {
                    let debt_comp_amount = price_provider
                        .convert_from_base(&debt.asset, total_debt_to_cover_in_base)?; //@audit takes 0, CLIENT_PRICES_READS, or 1+CLIENT_PRICES_READS reads


                    let debt_lp_amount = FixedI128::from_inner(debt.coeff.unwrap())
                        .recip_mul_int(debt_comp_amount)
                        .ok_or(Error::LiquidateMathError)?;  //@audit is this rounded down?

                    total_debt_to_cover_in_base = 0;

                    (debt_lp_amount, debt_comp_amount)
                };

            let underlying_asset = token::Client::new(env, &debt.asset);
            let debt_token = DebtTokenClient::new(env, debt_token_address);

            underlying_asset.transfer(liquidator, s_token_address, &debt_comp_to_transfer); //@audit if this is zero liquidation may revert here!
            
            debt_token.burn(who, &debt_lp_to_burn);

            let mut debt_token_supply = read_token_total_supply(env, debt_token_address); //@audit 1 read
            let s_token_supply = read_token_total_supply(env, s_token_address); //@audit 1 read

            debt_token_supply = debt_token_supply
                .checked_sub(debt_lp_to_burn)
                .ok_or(Error::LiquidateMathError)?;

            add_stoken_underlying_balance(env, s_token_address, debt_comp_to_transfer)?; //@audit 1 read + 1 write
            write_token_total_supply(env, debt_token_address, debt_token_supply)?;  //@audit 1 write
            write_token_balance(
                env,
                &debt_token.address,
                who,
                debt.lp_balance.unwrap() - debt_lp_to_burn,
            )?; //@audit 1 write

            recalculate_reserve_data(
                env,
                &debt.asset,
                &debt.reserve,
                s_token_supply,
                debt_token_supply,
            )?;
        }
    }

    user_configurator.write();

    Ok((debt_covered_in_base, total_liq_in_base))
}

#[allow(clippy::too_many_arguments)]
fn calc_liq_amount(
    price_provider: &mut PriceProvider,
    collat: &LiquidationAsset,
    hundred_percent: FixedI128,
    discount_percent: FixedI128,
    liq_bonus_percent: FixedI128,
    safe_collat_percent: FixedI128,
    initial_health_percent: FixedI128,
    total_collat_disc_in_base: i128,
    total_debt_in_base: i128,
) -> Result<i128, Error> {
    let safe_collat_in_base = safe_collat_percent
        .mul_int(total_collat_disc_in_base)
        .ok_or(Error::LiquidateMathError)?
        .checked_sub(total_debt_in_base)
        .ok_or(Error::LiquidateMathError)?;
    //@audit safe_collat_in_base = (safe_collat_percent * total_collat_disc_in_base) - total_debt_in_base
    let safe_discount_percent = discount_percent
        .checked_mul(initial_health_percent)
        .unwrap();
    //@audit safe_discount_percent = discount_percent * initial_health_percent
    let safe_discount_percent = discount_percent 
        .checked_add(liq_bonus_percent)
        .ok_or(Error::LiquidateMathError)?
        .checked_sub(hundred_percent)
        .ok_or(Error::LiquidateMathError)?
        .checked_sub(safe_discount_percent)
        .ok_or(Error::LiquidateMathError)?;
    //@audit safe_discount_percent = discount_percent + liq_bonus_percent - 100% - (discount_percent * initial_health_percent)
    //@audit TODO: is this WRONG? Something is troubling. The formula seems mistaken.  
    let liq_comp_amount = price_provider.convert_from_base(&collat.asset, safe_collat_in_base)?; 
    //@audit takes 0, CLIENT_PRICES_READS, or 1+CLIENT_PRICES_READS reads
    let liq_comp_amount = safe_discount_percent
        .recip_mul_int(liq_comp_amount)
        .ok_or(Error::LiquidateMathError)?;

    Ok(if liq_comp_amount.is_negative() {
        collat.comp_balance
    } else {
        collat.comp_balance.min(liq_comp_amount)
    })
}
