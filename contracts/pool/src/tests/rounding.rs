use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::tests::sut::{fill_pool, init_pool};

#[test]
fn rounding_deposit_withdraw() {
    extern crate std;

    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, _borrower, debt_config) = fill_pool(&env, &sut, true);
    let token_address = debt_config.token.address.clone();

    let attacker = Address::generate(&env);
    sut.reserves[1]
        .token_admin
        .mint(&attacker, &100_000_000_000);

    std::println!("collat coeff {:?}", sut.pool.collat_coeff(&token_address));
    std::println!("debt coeff {:?}", sut.pool.debt_coeff(&token_address));
    // i = 1 will panic with s-token: invalid mint amount, cause mint_amount would be equal to 0
    for i in 2..101 {
        env.budget().reset_unlimited();

        let balance_before = sut.reserves[1].token.balance(&attacker);
        let s_balance_before = sut.reserves[1].s_token().balance(&attacker);

        sut.pool.deposit(&attacker, &token_address, &i);

        let s_balance_after_deposit = sut.reserves[1].s_token().balance(&attacker);

        let s_token_balance = sut.reserves[1].s_token().balance(&attacker);
        if s_token_balance == 0 || s_token_balance >= i {
            std::println!("input {:?}, output {:?}", i, s_token_balance);
            panic!();
        }

        sut.pool.withdraw(&attacker, &token_address, &i, &attacker);

        let s_balance_after_withdraw = sut.reserves[1].s_token().balance(&attacker);

        if s_balance_after_deposit <= s_balance_before
            || s_balance_after_withdraw > s_balance_after_deposit
        {
            std::println!("{:?}: s_balance_before {:?}, s_balance_after_deposit {:?}, s_balance_after_withdraw {:?}",
            i,
            s_balance_before,
            s_balance_after_deposit,
            s_balance_after_withdraw);
            panic!();
        }

        let balance_after = sut.reserves[1].token.balance(&attacker);

        if balance_after > balance_before {
            std::println!(
                "{:?}: balance_before: {:?} balance_after: {:?}",
                i,
                balance_before,
                balance_after
            );
            panic!();
        }
    }
}

#[test]
fn rounding_borrow_repay() {
    extern crate std;

    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, _borrower, _debt_config) = fill_pool(&env, &sut, true);
    let token_address = sut.reserves[1].token.address.clone();

    let attacker = Address::generate(&env);
    sut.reserves[0]
        .token_admin
        .mint(&attacker, &100_000_000_000);
    sut.pool
        .deposit(&attacker, &sut.reserves[0].token.address, &100_000_000_000);

    std::println!("collat coeff {:?}", sut.pool.collat_coeff(&token_address));
    std::println!("debt coeff {:?}", sut.pool.debt_coeff(&token_address));
    // i = 1 will panic with zero or negative amount is not allowed, cause mint_amount would be equal to 0
    for i in 2..101 {
        env.budget().reset_unlimited();

        let balance_before = sut.reserves[1].token.balance(&attacker);
        let d_balance_before = sut.reserves[1].debt_token().balance(&attacker);

        sut.pool.borrow(&attacker, &token_address, &i);

        let d_balance_after_borrow = sut.reserves[1].debt_token().balance(&attacker);

        if d_balance_after_borrow == 0 {
            std::println!("input {:?}, output {:?}", i, d_balance_after_borrow);
            panic!();
        }

        sut.pool.repay(&attacker, &token_address, &i);

        let d_balance_after_repay = sut.reserves[1].debt_token().balance(&attacker);

        if d_balance_after_borrow <= d_balance_before
            || d_balance_after_repay == d_balance_after_borrow
        {
            std::println!("{:?}: d_balance_before {:?}, d_balance_after_borrow {:?}, d_balance_after_repay {:?}",
            i,
            d_balance_before,
            d_balance_after_borrow,
            d_balance_after_repay);
            panic!();
        }

        let balance_after = sut.reserves[1].token.balance(&attacker);

        if balance_after > balance_before {
            std::println!(
                "{:?}: balance_before: {:?} balance_after: {:?}",
                i,
                balance_before,
                balance_after
            );
            panic!();
        }
    }
}
