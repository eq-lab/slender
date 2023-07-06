use soroban_sdk::{contracttype, Address, Env};

#[derive(Clone)]
#[contracttype]
pub struct AllowanceDataKey {
    pub from: Address,
    pub spender: Address,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Allowance(AllowanceDataKey),
    Treasury,
}

pub fn write_treasury(e: &Env, treasury: &Address) {
    e.storage().set(&DataKey::Treasury, treasury);
}

pub fn read_treasury(e: &Env) -> Address {
    e.storage().get_unchecked(&DataKey::Treasury).unwrap()
}

pub fn read_allowance(e: &Env, from: Address, spender: Address) -> i128 {
    let key = DataKey::Allowance(AllowanceDataKey { from, spender });
    if let Some(allowance) = e.storage().get(&key) {
        allowance.unwrap()
    } else {
        0
    }
}

pub fn write_allowance(e: &Env, from: Address, spender: Address, amount: i128) {
    let key = DataKey::Allowance(AllowanceDataKey { from, spender });
    e.storage().set(&key, &amount);
}
