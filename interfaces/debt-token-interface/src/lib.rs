#![deny(warnings)]
#![no_std]

use soroban_sdk::{contractclient, contractspecfn, Address, BytesN, Env, String};
pub struct Spec;

#[contractspecfn(name = "Spec", export = false)]
#[contractclient(name = "DebtTokenClient")]
pub trait DebtTokenTrait {
    fn initialize(e: Env, name: String, symbol: String, pool: Address, underlying_asset: Address);

    fn upgrade(env: Env, new_wasm_hash: BytesN<32>);

    fn version() -> u32;

    fn balance(env: Env, id: Address) -> i128;

    fn spendable_balance(env: Env, id: Address) -> i128;

    fn authorized(env: Env, id: Address) -> bool;

    fn burn(env: Env, from: Address, amount: i128);

    fn burn_from(env: Env, spender: Address, from: Address, amount: i128);

    fn set_authorized(env: Env, id: Address, authorize: bool);

    fn mint(env: Env, to: Address, amount: i128);

    fn clawback(env: Env, from: Address, amount: i128);

    fn decimals(env: Env) -> u32;

    fn name(env: Env) -> String;

    fn symbol(env: Env) -> String;

    fn total_supply(env: Env) -> i128;
}
