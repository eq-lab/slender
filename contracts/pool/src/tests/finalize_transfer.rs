use crate::tests::sut::{fill_pool, init_pool, set_time};
use pool_interface::types::pool_config::PoolConfig;
use soroban_sdk::Env;

use super::sut::{create_s_token_contract, create_token_contract};

#[test]
fn finalize_transfer_should_change_in_pool_balance() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env, false);
    let (lender, borrower, _debt_token_reserve) = fill_pool(&env, &sut, true);
    let token_client = &sut.reserves[0].token;
    let s_token_client = sut.reserves[0].s_token();

    let lender_balance_before = s_token_client.balance(&lender);
    let borrower_balance_before = s_token_client.balance(&borrower);
    let lender_in_pool_before = sut.pool.balance(&lender, &s_token_client.address);
    let borrower_in_pool_before = sut.pool.balance(&borrower, &s_token_client.address);
    let s_token_supply = s_token_client.total_supply();
    sut.pool.finalize_transfer(
        &token_client.address,
        &lender,
        &borrower,
        &1,
        &lender_balance_before,
        &borrower_balance_before,
        &s_token_supply,
    );

    let lender_in_pool_after = sut.pool.balance(&lender, &s_token_client.address);
    let borrower_in_pool_after = sut.pool.balance(&borrower, &s_token_client.address);

    assert_eq!(lender_in_pool_before - lender_in_pool_after, 1);
    assert_eq!(borrower_in_pool_after - borrower_in_pool_before, 1);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn finalize_transfer_should_fail_if_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env, false);
    let (lender, borrower, _debt_token_reserve) = fill_pool(&env, &sut, true);
    let token_client = &sut.reserves[0].token;
    let s_token_client = sut.reserves[0].s_token();
    sut.pool.set_pause(&true);

    let lender_balance_before = s_token_client.balance(&lender);
    let borrower_balance_before = s_token_client.balance(&borrower);
    let s_token_supply = s_token_client.total_supply();
    sut.pool.finalize_transfer(
        &token_client.address,
        &lender,
        &borrower,
        &1,
        &lender_balance_before,
        &borrower_balance_before,
        &s_token_supply,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Auth, InvalidAction)")]
fn should_fail_when_transfering_unknown_asset() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env, false);
    let (lender, borrower, _debt_token_reserve) = fill_pool(&env, &sut, true);
    let token_client = &sut.reserves[0].token;
    let unknown_s_token = create_s_token_contract(&env, &sut.pool.address, &token_client.address);
    unknown_s_token.mint(&lender, &100);
    unknown_s_token.transfer(&lender, &borrower, &1);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #100)")]
fn should_fail_when_on_no_reserve() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env, false);
    let (lender, borrower, _debt_token_reserve) = fill_pool(&env, &sut, true);
    // let token_client = &sut.reserves[0].token;
    let s_token_client = sut.reserves[0].s_token();
    let (unknown_token, _) = create_token_contract(&env, &sut.token_admin);

    let s_token_supply = s_token_client.total_supply();
    let lender_balance_before = s_token_client.balance(&lender);
    let borrower_balance_before = s_token_client.balance(&borrower);

    sut.pool.finalize_transfer(
        &unknown_token.address,
        &lender,
        &borrower,
        &1,
        &lender_balance_before,
        &borrower_balance_before,
        &s_token_supply,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #101)")]
fn finalize_transfer_should_fail_if_reserve_is_not_active() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env, false);
    let (lender, borrower, _debt_token_reserve) = fill_pool(&env, &sut, true);
    let token_client = &sut.reserves[0].token;
    let s_token_client = sut.reserves[0].s_token();

    sut.pool.set_reserve_status(&token_client.address, &false);

    let lender_balance_before = s_token_client.balance(&lender);
    let borrower_balance_before = s_token_client.balance(&borrower);
    let s_token_supply = s_token_client.total_supply();
    sut.pool.finalize_transfer(
        &token_client.address,
        &lender,
        &borrower,
        &1,
        &lender_balance_before,
        &borrower_balance_before,
        &s_token_supply,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #110)")]
fn finalize_transfer_should_fail_if_calling_on_rwa() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env, false);
    let (lender, borrower, _debt_token_reserve) = fill_pool(&env, &sut, true);
    let token_client = &sut.reserves[3].token;
    let s_token_client = sut.reserves[0].s_token();

    let lender_balance_before = s_token_client.balance(&lender);
    let borrower_balance_before = s_token_client.balance(&borrower);
    let s_token_supply = s_token_client.total_supply();
    sut.pool.finalize_transfer(
        &token_client.address,
        &lender,
        &borrower,
        &1,
        &lender_balance_before,
        &borrower_balance_before,
        &s_token_supply,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #204)")]
fn finalize_transfer_should_fail_if_receiver_has_debt_in_same_asset() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env, false);
    let (lender, borrower, debt_token_reserve) = fill_pool(&env, &sut, true);
    let s_token_client = sut.reserves[0].s_token();

    let lender_balance_before = s_token_client.balance(&lender);
    let borrower_balance_before = s_token_client.balance(&borrower);
    let s_token_supply = s_token_client.total_supply();
    sut.pool.finalize_transfer(
        &debt_token_reserve.token.address,
        &lender,
        &borrower,
        &1,
        &lender_balance_before,
        &borrower_balance_before,
        &s_token_supply,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #303)")]
fn finalize_transfer_should_fail_if_transfers_with_underflow() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env, false);
    let (lender, borrower, _debt_token_reserve) = fill_pool(&env, &sut, true);
    let s_token_client = sut.reserves[0].s_token();
    let token_client = &sut.reserves[0].token;

    let lender_balance_before = s_token_client.balance(&lender);
    let borrower_balance_before = s_token_client.balance(&borrower);
    let s_token_supply = s_token_client.total_supply();
    sut.pool.finalize_transfer(
        &token_client.address,
        &lender,
        &borrower,
        &(i128::MAX),
        &lender_balance_before,
        &borrower_balance_before,
        &s_token_supply,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #301)")]
fn finalize_transfer_should_fail_if_npv_fail_bellow_initial_health() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env, false);
    let (lender, borrower, _debt_token_reserve) = fill_pool(&env, &sut, true);
    let s_token_client = sut.reserves[0].s_token();
    let token_client = &sut.reserves[0].token;

    let lender_balance_before = s_token_client.balance(&lender);
    let borrower_balance_before = s_token_client.balance(&borrower);
    let s_token_supply = s_token_client.total_supply();
    sut.pool.finalize_transfer(
        &token_client.address,
        &borrower,
        &lender,
        &(borrower_balance_before - 1),
        &lender_balance_before,
        &borrower_balance_before,
        &s_token_supply,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #205)")]
fn rwa_fail_when_exceed_assets_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env, false);
    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 0,
        timestamp_window: 20,
        user_assets_limit: 2,
        min_collat_amount: 0,
        min_debt_amount: 0,
        liquidation_protocol_fee: 0,
    });

    let (lender, borrower, _debt_token_reserve) = fill_pool(&env, &sut, true);
    let token_client = &sut.reserves[2].token;
    let s_token_client = sut.reserves[2].s_token();

    let lender_balance_before = s_token_client.balance(&lender);
    let borrower_balance_before = s_token_client.balance(&borrower);
    let s_token_supply = s_token_client.total_supply();
    sut.pool.finalize_transfer(
        &token_client.address,
        &lender,
        &borrower,
        &1,
        &lender_balance_before,
        &borrower_balance_before,
        &s_token_supply,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #207)")]
fn should_fail_when_collat_lt_min_position_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let sut = init_pool(&env, false);
    sut.pool.set_pool_configuration(&PoolConfig {
        base_asset_address: sut.reserves[0].token.address.clone(),
        base_asset_decimals: sut.reserves[0].token.decimals(),
        flash_loan_fee: 5,
        initial_health: 0,
        timestamp_window: 20,
        user_assets_limit: 3,
        min_collat_amount: 600_000,
        min_debt_amount: 0,
        liquidation_protocol_fee: 0,
    });

    let (lender, borrower, _debt_token_reserve) = fill_pool(&env, &sut, true);
    let token_client = &sut.reserves[0].token;
    let s_token_client = sut.reserves[0].s_token();

    let lender_balance_before = s_token_client.balance(&lender);
    let borrower_balance_before = s_token_client.balance(&borrower);
    let s_token_supply = s_token_client.total_supply();
    sut.pool.finalize_transfer(
        &token_client.address,
        &borrower,
        &lender,
        &500_000,
        &borrower_balance_before,
        &lender_balance_before,
        &s_token_supply,
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #6)")]
fn should_fail_in_grace_period() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (lender, borrower, _) = fill_pool(&env, &sut, false);

    let token_client = &sut.reserves[0].token;
    let s_token_client = sut.reserves[0].s_token();

    let lender_balance_before = s_token_client.balance(&lender);
    let borrower_balance_before = s_token_client.balance(&borrower);
    let lender_in_pool_before = sut.pool.balance(&lender, &s_token_client.address);
    let borrower_in_pool_before = sut.pool.balance(&borrower, &s_token_client.address);
    let s_token_supply = s_token_client.total_supply();
    sut.pool.finalize_transfer(
        &token_client.address,
        &lender,
        &borrower,
        &1,
        &lender_balance_before,
        &borrower_balance_before,
        &s_token_supply,
    );

    let lender_in_pool_after = sut.pool.balance(&lender, &s_token_client.address);
    let borrower_in_pool_after = sut.pool.balance(&borrower, &s_token_client.address);

    assert_eq!(lender_in_pool_before - lender_in_pool_after, 1);
    assert_eq!(borrower_in_pool_after - borrower_in_pool_before, 1);

    sut.pool.set_pause(&true);
    sut.pool.set_pause(&false);
    sut.pool.finalize_transfer(
        &token_client.address,
        &lender,
        &borrower,
        &1,
        &lender_balance_before,
        &borrower_balance_before,
        &s_token_supply,
    );
}

#[test]
fn should_not_fail_after_grace_period() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (lender, borrower, _) = fill_pool(&env, &sut, false);

    let pause_info = sut.pool.pause_info();
    let start = env.ledger().timestamp();
    let gap = 500;

    let token_client = &sut.reserves[0].token;
    let s_token_client = sut.reserves[0].s_token();

    let lender_balance_before = s_token_client.balance(&lender);
    let borrower_balance_before = s_token_client.balance(&borrower);
    let lender_in_pool_before = sut.pool.balance(&lender, &s_token_client.address);
    let borrower_in_pool_before = sut.pool.balance(&borrower, &s_token_client.address);
    let s_token_supply = s_token_client.total_supply();
    sut.pool.finalize_transfer(
        &token_client.address,
        &lender,
        &borrower,
        &1,
        &lender_balance_before,
        &borrower_balance_before,
        &s_token_supply,
    );

    let lender_in_pool_after = sut.pool.balance(&lender, &s_token_client.address);
    let borrower_in_pool_after = sut.pool.balance(&borrower, &s_token_client.address);

    assert_eq!(lender_in_pool_before - lender_in_pool_after, 1);
    assert_eq!(borrower_in_pool_after - borrower_in_pool_before, 1);

    sut.pool.set_pause(&true);
    set_time(&env, &sut, start + gap, false);
    sut.pool.set_pause(&false);
    set_time(
        &env,
        &sut,
        start + gap + pause_info.grace_period_secs,
        false,
    );

    let lender_balance_before = lender_balance_before - 1;
    let borrower_balance_before = borrower_balance_before + 1;
    let lender_in_pool_before = sut.pool.balance(&lender, &s_token_client.address);
    let borrower_in_pool_before = sut.pool.balance(&borrower, &s_token_client.address);
    let s_token_supply = s_token_client.total_supply();
    sut.pool.finalize_transfer(
        &token_client.address,
        &lender,
        &borrower,
        &1,
        &lender_balance_before,
        &borrower_balance_before,
        &s_token_supply,
    );

    let lender_in_pool_after = sut.pool.balance(&lender, &s_token_client.address);
    let borrower_in_pool_after = sut.pool.balance(&borrower, &s_token_client.address);

    assert_eq!(lender_in_pool_before - lender_in_pool_after, 1);
    assert_eq!(borrower_in_pool_after - borrower_in_pool_before, 1);
}
