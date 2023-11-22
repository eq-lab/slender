use soroban_sdk::{Address, Env};

use crate::require_positive_amount;
use crate::storage::{
    is_authorized, read_balance, read_total_supply, write_balance, write_total_supply,
};

pub fn receive_balance(e: &Env, addr: Address, amount: i128) {
    require_positive_amount(amount);
    let balance = read_balance(e, addr.clone());
    if !is_authorized(e, addr.clone()) {
        panic!("can't receive when deauthorized");
    }
    write_balance(e, addr, balance.checked_add(amount).expect("no overflow"));
}

pub fn spend_balance(e: &Env, addr: Address, amount: i128) {
    require_positive_amount(amount);
    let balance = read_balance(e, addr.clone());
    if !is_authorized(e, addr.clone()) {
        panic!("can't spend when deauthorized");
    }
    if balance < amount {
        panic!("insufficient balance");
    }
    write_balance(e, addr, balance - amount);
}

pub fn add_total_supply(e: &Env, amount: i128) {
    let total_supply = read_total_supply(e)
        .checked_add(amount)
        .expect("no overflow");
    if total_supply.is_negative() {
        panic!("negative total supply");
    }

    write_total_supply(e, total_supply);
}
