#![deny(warnings)]
#![no_std]

use soroban_sdk::{contractclient, contractspecfn, Address, Bytes, Env};
pub struct Spec;

/// Interface for SToken
#[contractspecfn(name = "Spec", export = false)]
#[contractclient(name = "STokenClient")]
pub trait STokenTrait {
    fn initialize(
        e: Env,
        decimal: u32,
        name: Bytes,
        symbol: Bytes,
        pool: Address,
        treasury: Address,
        underlying_asset: Address,
    );

    fn allowance(e: Env, from: Address, spender: Address) -> i128;

    fn increase_allowance(e: Env, from: Address, spender: Address, amount: i128);

    fn decrease_allowance(e: Env, from: Address, spender: Address, amount: i128);

    fn balance(e: Env, id: Address) -> i128;

    fn spendable_balance(e: Env, id: Address) -> i128;

    fn authorized(e: Env, id: Address) -> bool;

    fn transfer(e: Env, from: Address, to: Address, amount: i128);

    fn transfer_from(e: Env, spender: Address, from: Address, to: Address, amount: i128);

    fn burn(
        e: Env,
        from: Address,
        amount_to_burn: i128,
        amount_to_withdraw: i128,
        to: Address,
    ) -> i128;

    fn burn_from(e: Env, spender: Address, from: Address, amount: i128);

    fn clawback(e: Env, from: Address, amount: i128);

    fn set_authorized(e: Env, id: Address, authorize: bool);

    fn mint(e: Env, to: Address, amount: i128) -> i128;

    fn decimals(e: Env) -> u32;

    fn name(e: Env) -> Bytes;

    fn symbol(e: Env) -> Bytes;

    fn total_supply(e: Env) -> i128;

    fn mint_to_treasury(e: Env, amount: i128);

    fn transfer_on_liquidation(e: Env, from: Address, to: Address, amount: i128);

    fn transfer_underlying_to(e: Env, to: Address, amount: i128);

    fn underlying_asset(e: Env) -> Address;

    fn treasury(e: Env) -> Address;

    fn pool(e: Env) -> Address;

    fn underlying_balance(e: Env, user: Address) -> i128;

    fn underlying_total_supply(e: Env) -> i128;
}
