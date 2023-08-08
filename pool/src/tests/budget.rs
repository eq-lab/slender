#![cfg(test)]
extern crate std;

use super::{
    set_as_collateral::init_with_debt,
    sut::{create_price_feed_contract, fill_pool, fill_pool_three, init_pool, DAY},
};
use crate::{
    tests::sut::{create_pool_contract, create_s_token_contract, create_token_contract},
    LendingPool,
};
use pool_interface::{CollateralParamsInput, IRParams, InitReserveInput, LendingPoolClient};
use price_feed_interface::PriceFeedClient;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, Env,
};
use std::fs::OpenOptions;
use std::io::prelude::*;

#[test]
fn should_measure_account_position_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, _, _) = fill_pool_three(&env, &sut);

    measure_budget(&env, nameof(should_measure_account_position_budget), || {
        sut.pool.account_position(&borrower);
    });
}

#[test]
fn should_measure_borrow_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    measure_budget(&env, nameof(should_measure_borrow_budget), || {
        sut.pool.borrow(&borrower, &token_address, &20_000_000);
    });
}

#[test]
fn should_measure_collat_coeff_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, _, _, debt_config) = fill_pool_three(&env, &sut);
    let debt_token = debt_config.token.address.clone();

    measure_budget(&env, nameof(should_measure_collat_coeff_budget), || {
        sut.pool.collat_coeff(&debt_token);
    });
}

#[test]
fn should_measure_configure_as_collateral_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let asset_address = sut.token().address.clone();
    let decimals = sut.s_token().decimals();
    let params = CollateralParamsInput {
        liq_bonus: 11_000,
        liq_cap: 100_000_000 * 10_i128.pow(decimals),
        util_cap: 9_000,
        discount: 6_000,
    };

    measure_budget(
        &env,
        nameof(should_measure_configure_as_collateral_budget),
        || {
            sut.pool
                .configure_as_collateral(&asset_address.clone(), &params.clone());
        },
    );
}

#[test]
fn should_measure_debt_coeff_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, _, _, debt_config) = fill_pool_three(&env, &sut);
    let debt_token = debt_config.token.address.clone();

    measure_budget(&env, nameof(should_measure_debt_coeff_budget), || {
        sut.pool.debt_coeff(&debt_token);
    });
}

#[test]
fn should_measure_deposit_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let user = Address::random(&env);
    let token_address = sut.token().address.clone();

    sut.token_admin().mint(&user, &10_000_000_000);

    measure_budget(&env, nameof(should_measure_deposit_budget), || {
        sut.pool.deposit(&user, &token_address, &5_000_000_000)
    });
}

#[test]
fn should_measure_enable_borrowing_on_reserve_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let asset = sut.token().address.clone();

    measure_budget(
        &env,
        nameof(should_measure_enable_borrowing_on_reserve_budget),
        || {
            sut.pool.enable_borrowing_on_reserve(&asset, &true);
        },
    );
}

#[test]
fn should_measure_get_reserve_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let asset = sut.token().address.clone();

    measure_budget(&env, nameof(should_measure_get_reserve_budget), || {
        sut.pool.get_reserve(&asset);
    });
}

#[test]
fn should_measure_init_reserve_budget() {
    let env = Env::default();
    env.mock_all_auths();

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

    measure_budget(&env, nameof(should_measure_init_reserve_budget), || {
        pool.init_reserve(
            &underlying_token.address.clone(),
            &init_reserve_input.clone(),
        );
    });
}

#[test]
fn should_measure_ir_params_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    measure_budget(&env, nameof(should_measure_ir_params_budget), || {
        sut.pool.ir_params();
    });
}

#[test]
fn should_measure_liquidate_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, liquidator, _) = fill_pool_three(&env, &sut);

    sut.pool.liquidate(&liquidator, &borrower, &true);

    measure_budget(&env, nameof(should_measure_liquidate_budget), || {
        sut.pool.ir_params();
    });
}

#[test]
fn should_measure_paused_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    measure_budget(&env, nameof(should_measure_paused_budget), || {
        sut.pool.paused();
    });
}

#[test]
fn should_measure_price_feed_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    measure_budget(&env, nameof(should_measure_price_feed_budget), || {
        sut.pool.price_feed(&sut.token().address);
    });
}

#[test]
fn should_measure_repay_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, true);
    let debt_token = &debt_config.token.address;

    env.ledger().with_mut(|li| li.timestamp = DAY);

    measure_budget(&env, nameof(should_measure_repay_budget), || {
        sut.pool.deposit(&borrower, &debt_token.clone(), &i128::MAX);
    });
}

#[test]
fn should_measure_set_as_collateral_budget() {
    let env = Env::default();
    env.mock_all_auths();
    let (sut, user, (_, _), (collat_token, _)) = init_with_debt(&env);

    sut.reserves[2].token_admin.mint(&user, &2_000_000_000);
    sut.pool
        .deposit(&user, &sut.reserves[2].token_admin.address, &2_000_000_000);

    measure_budget(
        &env,
        nameof(should_measure_set_as_collateral_budget),
        || {
            sut.pool.set_as_collateral(&user, &collat_token, &false);
        },
    );
}

#[test]
fn should_measure_set_ir_params_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    let ir_params_input = IRParams {
        alpha: 144,
        initial_rate: 201,
        max_rate: 50_001,
        scaling_coeff: 9_001,
    };

    measure_budget(&env, nameof(should_measure_set_ir_params_budget), || {
        sut.pool.set_ir_params(&ir_params_input);
    });
}

#[test]
fn should_measure_set_pause_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);

    measure_budget(&env, nameof(should_measure_set_pause_budget), || {
        sut.pool.set_pause(&true);
    });
}

#[test]
fn should_measure_set_price_feed_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::random(&env);
    let asset_1 = Address::random(&env);
    let asset_2 = Address::random(&env);

    let pool: LendingPoolClient<'_> = create_pool_contract(&env, &admin);
    let price_feed: PriceFeedClient<'_> = create_price_feed_contract(&env);
    let assets = vec![&env, asset_1.clone(), asset_2.clone()];

    measure_budget(&env, nameof(should_measure_set_price_feed_budget), || {
        pool.set_price_feed(&price_feed.address.clone(), &assets.clone());
    });
}

#[test]
fn should_measure_set_reserve_status_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let asset = sut.token().address.clone();

    measure_budget(
        &env,
        nameof(should_measure_set_reserve_status_budget),
        || {
            sut.pool.set_reserve_status(&asset, &true);
        },
    );
}

#[test]
fn should_measure_stoken_underlying_balance_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let lender = Address::random(&env);

    sut.reserves[0].token_admin.mint(&lender, &2_000_000_000);
    sut.pool
        .deposit(&lender, &sut.reserves[0].token.address, &1_000_000_000);

    measure_budget(
        &env,
        nameof(should_measure_stoken_underlying_balance_budget),
        || {
            sut.pool
                .stoken_underlying_balance(&sut.reserves[0].s_token.address);
        },
    );
}

#[test]
fn should_measure_treasury_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let pool = LendingPoolClient::new(&env, &env.register_contract(None, LendingPool));

    let admin = Address::random(&env);
    let treasury = Address::random(&env);

    pool.initialize(
        &admin,
        &treasury,
        &IRParams {
            alpha: 143,
            initial_rate: 200,
            max_rate: 50_000,
            scaling_coeff: 9_000,
        },
    );

    measure_budget(&env, nameof(should_measure_treasury_budget), || {
        pool.treasury();
    });
}

#[test]
fn should_measure_user_configuration_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    measure_budget(
        &env,
        nameof(should_measure_user_configuration_budget),
        || {
            sut.pool.user_configuration(&borrower);
        },
    );
}

#[test]
fn should_measure_withdraw_budget() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    measure_budget(&env, nameof(should_measure_withdraw_budget), || {
        sut.pool
            .withdraw(&borrower, &sut.token().address, &10_000, &borrower);
    });
}

pub fn measure_budget(env: &Env, function: &str, callback: impl FnOnce()) {
    env.budget().reset_tracker();

    callback();

    let cpu = env.budget().cpu_instruction_cost();
    // TODO: bug in v0.9.2 (returns CPU cost)
    let memory = env.budget().memory_bytes_cost();

    let budget = &[
        std::format!("['{}'] = {{\n", function),
        std::format!("    \"cpu_cost\": {},\n", cpu),
        std::format!("    \"memory_cost\": {},\n", memory),
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

fn nameof<F>(_: F) -> &'static str
where
    F: Fn(),
{
    std::any::type_name::<F>()
}
