#![cfg(test)]
extern crate std;

use pool_interface::types::collateral_params_input::CollateralParamsInput;
use pool_interface::types::flash_loan_asset::FlashLoanAsset;
use pool_interface::types::ir_params::IRParams;
use pool_interface::types::oracle_asset::OracleAsset;
use pool_interface::types::price_feed::PriceFeed;
use pool_interface::types::price_feed_config_input::PriceFeedConfigInput;
use pool_interface::types::reserve_type::ReserveType;
use pool_interface::types::timestamp_precision::TimestampPrecision;
use pool_interface::LendingPoolClient;
use price_feed_interface::types::asset::Asset;
use price_feed_interface::types::price_data::PriceData;
use price_feed_interface::PriceFeedClient;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{vec, Address, Bytes, Env, IntoVal, Symbol, Val, Vec};
use std::fs::OpenOptions;
use std::io::prelude::*;

use crate::LendingPool;

use super::sut::{
    create_pool_contract, create_price_feed_contract, create_s_token_contract,
    create_token_contract, fill_pool, fill_pool_four, init_pool, DAY,
};
use super::upgrade::{debt_token_v2, pool_v2, s_token_v2};

const CPU_LIMIT: u64 = 100_000_000;
const MEM_LIMIT: u64 = 40 * 1024 * 1024;

macro_rules! function_name {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        &name[..name.len() - 3]
    }};
}

#[test]
fn account_position() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (lender, _, _) = fill_pool_four(&env, &sut);

    measure_budget(&env, function_name!(), || {
        sut.pool.account_position(&lender);
    });
}

#[test]
fn borrow() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, _) = fill_pool_four(&env, &sut);

    measure_budget(&env, function_name!(), || {
        sut.pool
            .borrow(&borrower, &sut.reserves[2].token.address, &20_000_000);
    });
}

#[test]
fn collat_coeff() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, _, _) = fill_pool_four(&env, &sut);

    measure_budget(&env, function_name!(), || {
        sut.pool.collat_coeff(&sut.reserves[2].token.address);
    });
}

#[test]
fn configure_as_collateral() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let asset_address = sut.token().address.clone();
    let decimals = sut.s_token().decimals();
    let params = CollateralParamsInput {
        liq_cap: 100_000_000 * 10_i128.pow(decimals),
        pen_order: 1,
        util_cap: 9_000,
        discount: 6_000,
    };

    measure_budget(&env, function_name!(), || {
        sut.pool
            .configure_as_collateral(&asset_address.clone(), &params.clone());
    });
}

#[test]
fn debt_coeff() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, _, _) = fill_pool_four(&env, &sut);

    measure_budget(&env, function_name!(), || {
        sut.pool.debt_coeff(&sut.reserves[2].token.address);
    });
}

#[test]
fn deposit() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, _) = fill_pool_four(&env, &sut);

    measure_budget(&env, function_name!(), || {
        sut.pool
            .deposit(&borrower, &&sut.reserves[0].token.address, &10_000_000)
    });
}

#[test]
fn enable_borrowing_on_reserve() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let asset = sut.token().address.clone();

    measure_budget(&env, function_name!(), || {
        sut.pool.enable_borrowing_on_reserve(&asset, &true);
    });
}

#[test]
fn get_reserve() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let asset = sut.token().address.clone();

    measure_budget(&env, function_name!(), || {
        sut.pool.get_reserve(&asset);
    });
}

#[test]
fn init_reserve() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);

    let (underlying_token, _) = create_token_contract(&env, &token_admin);
    let (debt_token, _) = create_token_contract(&env, &token_admin);

    let pool = create_pool_contract(&env, &admin, false);
    let s_token = create_s_token_contract(&env, &pool.address, &underlying_token.address);
    assert!(pool.get_reserve(&underlying_token.address).is_none());

    let init_reserve_input =
        ReserveType::Fungible(s_token.address.clone(), debt_token.address.clone());

    measure_budget(&env, function_name!(), || {
        pool.init_reserve(
            &underlying_token.address.clone(),
            &init_reserve_input.clone(),
        );
    });
}

#[test]
fn ir_params() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, _, _) = fill_pool_four(&env, &sut);

    measure_budget(&env, function_name!(), || {
        sut.pool.ir_params();
    });
}

#[test]
fn liquidate_receive_stoken_when_borrower_has_two_debts() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, _, borrower) = fill_pool_four(&env, &sut);

    env.ledger().with_mut(|li| li.timestamp = 4 * DAY);

    let liquidator = Address::generate(&env);

    for reserve in &sut.reserves {
        reserve.token_admin.mint(&liquidator, &100_000_000_000);
    }

    sut.pool
        .deposit(&liquidator, &sut.reserves[0].token.address, &100_000_000);
    sut.pool
        .borrow(&liquidator, &sut.reserves[1].token.address, &1_000_000_000);

    env.ledger().with_mut(|l| l.timestamp = 5 * DAY);

    sut.price_feed.init(
        &Asset::Stellar(sut.reserves[0].token.address.clone()),
        &vec![
            &env,
            PriceData {
                price: 110_000_000_000_000,
                timestamp: 0,
            },
        ],
    );

    measure_budget(&env, function_name!(), || {
        sut.pool.liquidate(&liquidator, &borrower, &true);
    });
}

#[test]
fn liquidate_receive_underlying_when_borrower_has_one_debt() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, _) = fill_pool_four(&env, &sut);
    sut.pool.set_initial_health(&100);

    sut.pool
        .borrow(&borrower, &sut.reserves[2].token.address, &4_990_400_000);

    env.ledger().with_mut(|li| li.timestamp = 4 * DAY);

    let liquidator = Address::generate(&env);

    sut.reserves[0]
        .token_admin
        .mint(&liquidator, &100_000_000_000);

    sut.reserves[2]
        .token_admin
        .mint(&liquidator, &100_000_000_000);

    sut.pool
        .deposit(&liquidator, &sut.reserves[0].token.address, &10_000_000_000);
    sut.pool
        .borrow(&liquidator, &sut.reserves[2].token.address, &1_000_000_000);
    sut.pool
        .borrow(&liquidator, &sut.reserves[1].token.address, &1_000_000_000);

    env.ledger().with_mut(|l| l.timestamp = 5 * DAY);

    sut.price_feed.init(
        &Asset::Stellar(sut.reserves[2].token.address.clone()),
        &vec![
            &env,
            PriceData {
                price: 12_000_000_000_000_000,
                timestamp: 0,
            },
        ],
    );

    measure_budget(&env, function_name!(), || {
        sut.pool.liquidate(&liquidator, &borrower, &false);
    });
}

#[test]
fn liquidate_receive_underlying_when_borrower_has_two_debts() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, _, borrower) = fill_pool_four(&env, &sut);

    env.ledger().with_mut(|li| li.timestamp = 4 * DAY);

    let liquidator = Address::generate(&env);

    sut.reserves[0]
        .token_admin
        .mint(&liquidator, &1_000_000_000);

    sut.reserves[1]
        .token_admin
        .mint(&liquidator, &100_000_000_000);

    sut.pool
        .deposit(&liquidator, &sut.reserves[0].token.address, &100_000_000);
    sut.pool
        .borrow(&liquidator, &sut.reserves[2].token.address, &1_000_000_000);
    sut.pool
        .borrow(&liquidator, &sut.reserves[1].token.address, &1_000_000_000);

    env.ledger().with_mut(|l| l.timestamp = 5 * DAY);

    sut.price_feed.init(
        &Asset::Stellar(sut.reserves[0].token.address.clone()),
        &vec![
            &env,
            PriceData {
                price: 100_100_000_000_000,
                timestamp: 0,
            },
        ],
    );

    measure_budget(&env, function_name!(), || {
        sut.pool.liquidate(&liquidator, &borrower, &false);
    });
}

#[test]
fn paused() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    measure_budget(&env, function_name!(), || {
        sut.pool.paused();
    });
}

#[test]
fn price_feed() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    measure_budget(&env, function_name!(), || {
        sut.pool.price_feeds(&sut.token().address);
    });
}

#[test]
fn repay_full() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, _) = fill_pool_four(&env, &sut);

    measure_budget(&env, function_name!(), || {
        sut.pool
            .repay(&borrower, &sut.reserves[2].token.address, &i128::MAX);
    });
}

#[test]
fn repay_partial() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, _) = fill_pool_four(&env, &sut);

    measure_budget(&env, function_name!(), || {
        sut.pool
            .repay(&borrower, &sut.reserves[2].token.address, &1_000_000);
    });
}

#[test]
fn set_as_collateral() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, _) = fill_pool_four(&env, &sut);

    sut.pool
        .deposit(&borrower, &sut.reserves[1].token.address, &20_000_000_000);

    measure_budget(&env, function_name!(), || {
        sut.pool
            .set_as_collateral(&borrower, &sut.reserves[0].token.address, &false);
    });
}

#[test]
fn set_base_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let asset = Address::generate(&env);
    let sut = init_pool(&env, true);

    measure_budget(&env, function_name!(), || {
        sut.pool.set_base_asset(&asset, &7);
    });
}

#[test]
fn set_ir_params() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    let ir_params_input = IRParams {
        alpha: 144,
        initial_rate: 201,
        max_rate: 50_001,
        scaling_coeff: 9_001,
    };

    measure_budget(&env, function_name!(), || {
        sut.pool.set_ir_params(&ir_params_input);
    });
}

#[test]
fn set_pause() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    measure_budget(&env, function_name!(), || {
        sut.pool.set_pause(&true);
    });
}

#[test]
fn set_price_feed() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let asset_1 = Address::generate(&env);
    let asset_2 = Address::generate(&env);
    let asset_3 = Address::generate(&env);

    let pool = create_pool_contract(&env, &admin, false);
    let price_feed: PriceFeedClient<'_> = create_price_feed_contract(&env);

    let feed_inputs = Vec::from_array(
        &env,
        [
            PriceFeedConfigInput {
                asset: asset_1.clone(),
                asset_decimals: 7,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed.address.clone(),
                        feed_asset: OracleAsset::Stellar(asset_1),
                        feed_decimals: 14,
                        twap_records: 10,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
            PriceFeedConfigInput {
                asset: asset_2.clone(),
                asset_decimals: 9,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed.address.clone(),
                        feed_asset: OracleAsset::Stellar(asset_2),
                        feed_decimals: 16,
                        twap_records: 10,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
            PriceFeedConfigInput {
                asset: asset_3.clone(),
                asset_decimals: 9,
                feeds: vec![
                    &env,
                    PriceFeed {
                        feed: price_feed.address.clone(),
                        feed_asset: OracleAsset::Stellar(asset_3),
                        feed_decimals: 16,
                        twap_records: 10,
                        timestamp_precision: TimestampPrecision::Sec,
                    },
                ],
            },
        ],
    );

    measure_budget(&env, function_name!(), || {
        pool.set_price_feeds(&feed_inputs);
    });
}

#[test]
fn set_reserve_status() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let asset = sut.token().address.clone();

    measure_budget(&env, function_name!(), || {
        sut.pool.set_reserve_status(&asset, &true);
    });
}

#[test]
fn stoken_underlying_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let lender = Address::generate(&env);

    sut.reserves[0].token_admin.mint(&lender, &20_000_000);
    sut.pool
        .deposit(&lender, &sut.reserves[0].token.address, &10_000_000);

    measure_budget(&env, function_name!(), || {
        sut.pool
            .stoken_underlying_balance(&sut.reserves[0].s_token().address);
    });
}

#[test]
fn treasury() {
    let env = Env::default();
    env.mock_all_auths();

    let pool = LendingPoolClient::new(&env, &env.register_contract(None, LendingPool));
    let flash_loan_fee = 5;

    pool.initialize(
        &Address::generate(&env),
        &Address::generate(&env),
        &flash_loan_fee,
        &2_500,
        &IRParams {
            alpha: 143,
            initial_rate: 200,
            max_rate: 50_000,
            scaling_coeff: 9_000,
        },
    );

    measure_budget(&env, function_name!(), || {
        pool.treasury();
    });
}

#[test]
fn user_configuration() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    measure_budget(&env, function_name!(), || {
        sut.pool.user_configuration(&borrower);
    });
}

#[test]
fn withdraw_full() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, _) = fill_pool_four(&env, &sut);
    sut.pool
        .deposit(&borrower, &sut.reserves[1].token.address, &20_000_000_000);

    measure_budget(&env, function_name!(), || {
        sut.pool
            .withdraw(&borrower, &sut.token().address, &i128::MAX, &borrower);
    });
}

#[test]
fn withdraw_partial() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, _) = fill_pool_four(&env, &sut);

    sut.pool
        .deposit(&borrower, &sut.reserves[1].token.address, &20_000_000_000);

    measure_budget(&env, function_name!(), || {
        sut.pool
            .withdraw(&borrower, &sut.token().address, &10_000, &borrower);
    });
}

#[test]
fn flash_loan_fee() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    measure_budget(&env, function_name!(), || {
        sut.pool.flash_loan_fee();
    });
}

#[test]
fn set_flash_loan_fee() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    measure_budget(&env, function_name!(), || {
        sut.pool.set_flash_loan_fee(&15);
    });
}

#[test]
fn flash_loan_with_borrow() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    let _: Val = env.invoke_contract(
        &sut.flash_loan_receiver.address,
        &Symbol::new(&env, "initialize"),
        vec![&env, sut.pool.address.into_val(&env), false.into_val(&env)],
    );

    let loan_assets = Vec::from_array(
        &env,
        [
            FlashLoanAsset {
                asset: sut.reserves[0].token.address.clone(),
                amount: 10_000,
                borrow: false,
            },
            FlashLoanAsset {
                asset: sut.reserves[1].token.address.clone(),
                amount: 2_000_000,
                borrow: true,
            },
            FlashLoanAsset {
                asset: sut.reserves[2].token.address.clone(),
                amount: 3_000_000,
                borrow: true,
            },
        ],
    );

    measure_budget(&env, function_name!(), || {
        sut.pool.flash_loan(
            &borrower,
            &sut.flash_loan_receiver.address,
            &loan_assets,
            &Bytes::new(&env),
        );
    });
}

#[test]
fn flash_loan_without_borrow() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    let _: Val = env.invoke_contract(
        &sut.flash_loan_receiver.address,
        &Symbol::new(&env, "initialize"),
        vec![&env, sut.pool.address.into_val(&env), false.into_val(&env)],
    );

    let loan_assets = Vec::from_array(
        &env,
        [
            FlashLoanAsset {
                asset: sut.reserves[0].token.address.clone(),
                amount: 10_000,
                borrow: false,
            },
            FlashLoanAsset {
                asset: sut.reserves[1].token.address.clone(),
                amount: 2_000_000,
                borrow: false,
            },
            FlashLoanAsset {
                asset: sut.reserves[2].token.address.clone(),
                amount: 3_000_000,
                borrow: false,
            },
        ],
    );

    measure_budget(&env, function_name!(), || {
        sut.pool.flash_loan(
            &borrower,
            &sut.flash_loan_receiver.address,
            &loan_assets,
            &Bytes::new(&env),
        );
    });
}

#[test]
fn upgrade() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let pool_v2_wasm = env.deployer().upload_contract_wasm(pool_v2::WASM);

    measure_budget(&env, function_name!(), || {
        sut.pool.upgrade(&pool_v2_wasm);
    });
}

#[test]
fn upgrade_s_token() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let asset = sut.reserves[0].token.address.clone();

    let s_token_v2_wasm = env.deployer().upload_contract_wasm(s_token_v2::WASM);

    measure_budget(&env, function_name!(), || {
        sut.pool.upgrade_s_token(&asset, &s_token_v2_wasm);
    });
}

#[test]
fn upgrade_debt_token() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let debt_token_v2_wasm = env.deployer().upload_contract_wasm(debt_token_v2::WASM);
    let asset = sut.reserves[0].token.address.clone();

    measure_budget(&env, function_name!(), || {
        sut.pool.upgrade_debt_token(&asset, &debt_token_v2_wasm);
    });
}

#[test]
fn s_token_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, from_borrower, _) = fill_pool(&env, &sut, true);
    sut.pool.deposit(
        &from_borrower,
        &sut.reserves[2].token.address,
        &1_000_000_000,
    );
    let to = Address::generate(&env);

    measure_budget(&env, function_name!(), || {
        sut.reserves[0]
            .s_token()
            .transfer(&from_borrower, &to, &100_000);
    });
}

fn measure_budget(env: &Env, function: &str, callback: impl FnOnce()) {
    let cpu_before = env.budget().cpu_instruction_cost();
    let memory_before = env.budget().memory_bytes_cost();

    callback();

    let cpu_after = env.budget().cpu_instruction_cost();
    let memory_after = env.budget().memory_bytes_cost();

    let cpu = cpu_after - cpu_before;
    let memory = memory_after - memory_before;

    let budget = &[
        std::format!("['{}'] = {{\n", function),
        std::format!("    \"cpu_cost\": {},\n", cpu),
        std::format!("    \"memory_cost\": {},\n", memory),
        std::format!("    \"cpu_limit_exceeded\": {},\n", cpu > CPU_LIMIT),
        std::format!("    \"memory_limit_exceeded\": {},\n", memory > MEM_LIMIT),
        std::format!("}}"),
    ]
    .concat();

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open("src/tests/snapshots/budget_utilization.snap")
        .unwrap();
    let result = writeln!(file, "{}", budget);

    if let Err(e) = result {
        panic!("Failed to write budget consumption: {}", e);
    }
}
