#![deny(warnings)]
#![no_std]

use soroban_sdk::{Address, Env};

pub mod balance;
pub mod storage;

pub fn verify_caller_is_pool(e: &Env) -> Address {
    let pool = crate::storage::read_pool(e);
    pool.require_auth();
    pool
}

pub fn require_nonnegative_amount(amount: i128) {
    if amount < 0 {
        panic!("negative amount is not allowed: {}", amount)
    }
}

pub fn require_positive_amount(amount: i128) {
    if amount <= 0 {
        panic!("zero or negative amount is not allowed: {}", amount)
    }
}
