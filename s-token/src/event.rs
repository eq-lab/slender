use soroban_sdk::{symbol_short, Address, Env, String, Symbol};

pub(crate) fn approve(e: &Env, from: Address, to: Address, amount: i128, expiration_ledger: u32) {
    let topics = (Symbol::new(e, "approve"), from, to);
    e.events().publish(topics, (amount, expiration_ledger));
}

pub(crate) fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
    let topics = (symbol_short!("transfer"), from, to);
    e.events().publish(topics, amount);
}

pub(crate) fn mint(e: &Env, admin: Address, to: Address, amount: i128) {
    let topics = (symbol_short!("mint"), admin, to);
    e.events().publish(topics, amount);
}

pub(crate) fn burn(e: &Env, from: Address, amount: i128) {
    let topics = (symbol_short!("burn"), from);
    e.events().publish(topics, amount);
}

pub(crate) fn initialized(
    e: &Env,
    underlying_asset: Address,
    pool: Address,
    treasury: Address,
    decimals: u32,
    name: String,
    symbol: String,
) {
    let topics = (symbol_short!("init"), underlying_asset, pool);
    e.events()
        .publish(topics, (treasury, decimals, name, symbol));
}
