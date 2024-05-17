use soroban_sdk::{Address, Env};

use crate::require_positive_amount;
use crate::storage::{
    is_authorized, read_balance, read_total_supply, write_balance, write_total_supply,
};

pub fn receive_balance(e: &Env, addr: Address, amount: i128) {
    require_positive_amount(amount);
    let balance = read_balance(e, addr.clone()); //@audit 1 read
    if !is_authorized(e, addr.clone()) { //@audit 1 read
        panic!("can't receive when deauthorized");
    }
    write_balance(e, addr, balance.checked_add(amount).expect("no overflow")); //@audit 1 write
} //@audit takes 2 read + 1 write

pub fn spend_balance(e: &Env, addr: Address, amount: i128) {
    require_positive_amount(amount);
    let balance = read_balance(e, addr.clone()); //@audit 1 read
    if !is_authorized(e, addr.clone()) { //@audit 1 read
        panic!("can't spend when deauthorized");
    }
    if balance < amount {
        panic!("insufficient balance");
    }
    write_balance(e, addr, balance - amount); //@audit 1 write
} //@audit takes 2 read + 1 write

pub fn add_total_supply(e: &Env, amount: i128) { //@audit should we not check authorization here?
    let total_supply = read_total_supply(e)
        .checked_add(amount)
        .expect("no overflow");
    if total_supply.is_negative() {
        panic!("negative total supply");
    } //@audit 1 read

    write_total_supply(e, total_supply); //@audit 1 write
} //@audit total 1 read + 1 write
