use crate::tests::sut::{
    create_pool_contract, create_price_feed_contract, create_s_token_contract,
    create_token_contract, init_pool,
};
use crate::*;
use price_feed_interface::PriceFeedClient;
use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{vec, IntoVal};

use super::sut::fill_pool;

extern crate std;

#[test]
fn init_reserve() {
    let env = Env::default();

    let admin = Address::random(&env);
    let token_admin = Address::random(&env);

    let (underlying_token, _) = create_token_contract(&env, &token_admin);
    let (debt_token, _) = create_token_contract(&env, &token_admin);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin);
    let s_token = create_s_token_contract(&env, &pool.address, &underlying_token.address);
    assert!(pool.get_reserve(&underlying_token.address).is_none());

    let init_reserve_input = InitReserveInput {
        s_token_address: s_token.address.clone(),
        debt_token_address: debt_token.address.clone(),
    };

    assert_eq!(
        pool.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &pool.address,
                fn_name: "init_reserve",
                args: (&underlying_token.address, init_reserve_input.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .init_reserve(&underlying_token.address, &init_reserve_input),
        ()
    );

    let reserve = pool.get_reserve(&underlying_token.address).unwrap();

    assert!(pool.get_reserve(&underlying_token.address).is_some());
    assert_eq!(init_reserve_input.s_token_address, reserve.s_token_address);
    assert_eq!(
        init_reserve_input.debt_token_address,
        reserve.debt_token_address
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn init_reserve_second_time() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let init_reserve_input = InitReserveInput {
        s_token_address: sut.s_token().address.clone(),
        debt_token_address: sut.debt_token().address.clone(),
    };

    //TODO: check error after soroban fix
    sut.pool
        .init_reserve(&sut.token().address, &init_reserve_input);

    // assert_eq!(
    //     sut.pool
    //         .try_init_reserve(&sut.token().address, &init_reserve_input)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::ReserveAlreadyInitialized
    // )
}

#[test]
fn init_reserve_when_pool_not_initialized() {
    let env = Env::default();

    let admin = Address::random(&env);
    let token_admin = Address::random(&env);

    let (underlying_token, _) = create_token_contract(&env, &token_admin);
    let (debt_token, _) = create_token_contract(&env, &token_admin);

    let pool: LendingPoolClient<'_> =
        LendingPoolClient::new(&env, &env.register_contract(None, LendingPool));
    let s_token = create_s_token_contract(&env, &pool.address, &underlying_token.address);
    assert!(pool.get_reserve(&underlying_token.address).is_none());

    let init_reserve_input = InitReserveInput {
        s_token_address: s_token.address.clone(),
        debt_token_address: debt_token.address.clone(),
    };

    assert_eq!(
        pool.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &pool.address,
                fn_name: "init_reserve",
                args: (&underlying_token.address, init_reserve_input.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .try_init_reserve(&underlying_token.address, &init_reserve_input)
        .unwrap_err()
        .unwrap(),
        Error::Uninitialized
    );
}

#[test]
fn set_ir_params() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let ir_params_input = IRParams {
        alpha: 144,
        initial_rate: 201,
        max_rate: 50_001,
        scaling_coeff: 9_001,
    };

    sut.pool.set_ir_params(&ir_params_input);

    let ir_params = sut.pool.ir_params().unwrap();

    assert_eq!(ir_params_input.alpha, ir_params.alpha);
    assert_eq!(ir_params_input.initial_rate, ir_params.initial_rate);
    assert_eq!(ir_params_input.max_rate, ir_params.max_rate);
    assert_eq!(ir_params_input.scaling_coeff, ir_params.scaling_coeff);
}

#[test]
fn set_price_feed() {
    let env = Env::default();

    let admin = Address::random(&env);
    let asset_1 = Address::random(&env);
    let asset_2 = Address::random(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin);
    let price_feed: PriceFeedClient<'_> = create_price_feed_contract(&env);
    let assets = vec![&env, asset_1.clone(), asset_2.clone()];

    assert!(pool.price_feed(&asset_1.clone()).is_none());
    assert!(pool.price_feed(&asset_2.clone()).is_none());

    assert_eq!(
        pool.mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &pool.address,
                fn_name: "set_price_feed",
                args: (&price_feed.address, assets.clone()).into_val(&env),
                sub_invokes: &[],
            },
        }])
        .set_price_feed(&price_feed.address, &assets.clone()),
        ()
    );

    assert_eq!(pool.price_feed(&asset_1).unwrap(), price_feed.address);
    assert_eq!(pool.price_feed(&asset_2).unwrap(), price_feed.address);
}

#[test]
#[should_panic(expected = "HostError: Error(Value, InvalidInput)")]
fn liquidity_cap_test() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let (lender, _borrower, debt_config) = fill_pool(&env, &sut, true);

    let token_one = 10_i128.pow(debt_config.token.decimals());
    let liq_bonus = 11000; //110%
    let liq_cap = 1_000_000 * 10_i128.pow(debt_config.token.decimals()); // 1M
    let discount = 6000; //60%
    let util_cap = 9000; //90%

    sut.pool.configure_as_collateral(
        &debt_config.token.address,
        &CollateralParamsInput {
            liq_bonus,
            liq_cap,
            discount,
            util_cap,
        },
    );

    //TODO: check error after soroban fix
    let deposit_amount = 1_000_000 * token_one;
    sut.pool
        .deposit(&lender, &debt_config.token.address, &deposit_amount);

    // assert_eq!(
    //     sut.pool
    //         .try_deposit(&lender, &debt_config.token.address, &deposit_amount)
    //         .unwrap_err()
    //         .unwrap(),
    //     Error::LiqCapExceeded
    // );
}

#[test]
fn stoken_balance_not_changed_when_direct_transfer_to_underlying_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let lender = Address::random(&env);

    sut.reserves[0].token_admin.mint(&lender, &2_000_000_000);
    sut.pool
        .deposit(&lender, &sut.reserves[0].token.address, &1_000_000_000);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[0].s_token.address);

    assert_eq!(s_token_underlying_supply, 1_000_000_000);

    sut.reserves[0]
        .token
        .transfer(&lender, &sut.reserves[0].s_token.address, &1_000_000_000);

    let s_token_underlying_supply = sut
        .pool
        .get_stoken_underlying_balance(&sut.reserves[0].s_token.address);

    assert_eq!(s_token_underlying_supply, 1_000_000_000);
}
