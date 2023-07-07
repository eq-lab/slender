#![deny(warnings)]
#![no_std]

mod event;

use common_token::{
    balance::{add_total_supply, receive_balance, spend_balance},
    check_nonnegative_amount,
    storage::*,
    verify_caller_is_pool,
};
use debt_token_interface::DebtTokenTrait;
use soroban_sdk::{contractimpl, Address, Bytes, Env};
use soroban_token_sdk::TokenMetadata;

pub struct DebtToken;

#[contractimpl]
impl DebtTokenTrait for DebtToken {
    fn initialize(
        e: Env,
        decimal: u32,
        name: Bytes,
        symbol: Bytes,
        pool: Address,
        underlying_asset: Address,
    ) {
        if decimal > u8::MAX.into() {
            panic!("Decimal must fit in a u8");
        }

        if has_pool(&e) {
            panic!("Already initialized")
        }

        write_pool(&e, &pool);

        write_metadata(
            &e,
            TokenMetadata {
                decimal,
                name: name.clone(),
                symbol: symbol.clone(),
            },
        );

        event::initialized(&e, underlying_asset, pool, decimal, name, symbol);
    }
    fn balance(env: Env, id: Address) -> i128 {
        read_balance(&env, id)
    }
    fn spendable_balance(env: Env, id: Address) -> i128 {
        Self::balance(env, id)
    }
    fn authorized(env: Env, id: Address) -> bool {
        is_authorized(&env, id)
    }
    fn burn(env: Env, from: Address, amount: i128) {
        check_nonnegative_amount(amount);
        verify_caller_is_pool(&env);

        Self::do_burn(&env, from.clone(), amount);

        event::burn(&env, from, amount);
    }
    fn burn_from(_env: Env, _spender: Address, _from: Address, _amount: i128) {
        unimplemented!();
    }
    fn set_authorized(e: Env, id: Address, authorize: bool) {
        verify_caller_is_pool(&e);

        write_authorization(&e, id.clone(), authorize);
        event::set_authorized(&e, id, authorize);
    }
    fn mint(env: Env, to: Address, amount: i128) {
        check_nonnegative_amount(amount);
        let pool = verify_caller_is_pool(&env);

        receive_balance(&env, to.clone(), amount);
        add_total_supply(&env, amount);
        event::mint(&env, pool, to, amount);
    }
    fn clawback(env: Env, from: Address, amount: i128) {
        check_nonnegative_amount(amount);
        verify_caller_is_pool(&env);

        spend_balance(&env, from.clone(), amount);
        add_total_supply(&env, amount.checked_neg().expect("no overflow"));
        event::clawback(&env, from, amount);
    }
    fn decimals(env: Env) -> u32 {
        read_decimal(&env)
    }

    fn name(env: Env) -> Bytes {
        read_name(&env)
    }

    fn symbol(env: Env) -> Bytes {
        read_symbol(&env)
    }

    fn total_supply(env: Env) -> i128 {
        read_total_supply(&env)
    }
}

impl DebtToken {
    fn do_burn(e: &Env, from: Address, amount: i128) {
        let balance = read_balance(e, from.clone());
        if !is_authorized(e, from.clone()) {
            panic!("can't spend when deauthorized");
        }
        write_balance(
            e,
            from,
            balance.checked_sub(amount).expect("sufficient balance"),
        );
        add_total_supply(e, amount.checked_neg().expect("no overflow"));
    }
}
