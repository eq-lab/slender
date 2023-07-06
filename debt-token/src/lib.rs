// #![deny(warnings)]
#![no_std]

mod event;
// mod storage;

// use crate::storage::*;
use common_token::{storage::*, verify_caller_is_pool};
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
        write_underlying_asset(&e, &underlying_asset);

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
        todo!();
    }
    fn burn_from(env: Env, spender: Address, from: Address, amount: i128) {
        todo!();
    }
    fn set_authorized(e: Env, id: Address, authorize: bool) {
        verify_caller_is_pool(&e);

        write_authorization(&e, id.clone(), authorize);
        event::set_authorized(&e, id, authorize);
    }
    fn mint(env: Env, to: Address, amount: i128, amount_to_borrow: i128) {
        todo!();
    }
    fn clawback(env: Env, from: Address, amount: i128) {
        todo!();
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
