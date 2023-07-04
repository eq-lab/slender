use soroban_sdk::{Address, Bytes, Env, Symbol};

pub(crate) fn increase_allowance(e: &Env, from: Address, to: Address, amount: i128) {
    let topics = (Symbol::new(e, "increase_allowance"), from, to);
    e.events().publish(topics, amount);
}

pub(crate) fn decrease_allowance(e: &Env, from: Address, to: Address, amount: i128) {
    let topics = (Symbol::new(e, "decrease_allowance"), from, to);
    e.events().publish(topics, amount);
}

pub(crate) fn transfer(e: &Env, from: Address, to: Address, amount: i128) {
    let topics = (Symbol::short("transfer"), from, to);
    e.events().publish(topics, amount);
}

pub(crate) fn mint(e: &Env, admin: Address, to: Address, amount: i128) {
    let topics = (Symbol::short("mint"), admin, to);
    e.events().publish(topics, amount);
}

pub(crate) fn clawback(e: &Env, from: Address, amount: i128) {
    let topics = (Symbol::short("clawback"), from);
    e.events().publish(topics, amount);
}

pub(crate) fn set_authorized(e: &Env, id: Address, authorize: bool) {
    let topics = (Symbol::new(e, "set_authorized"), id);
    e.events().publish(topics, authorize);
}

pub(crate) fn burn(e: &Env, from: Address, amount: i128) {
    let topics = (Symbol::short("burn"), from);
    e.events().publish(topics, amount);
}

pub(crate) fn initialized(
    e: &Env,
    underlying_asset: Address,
    pool: Address,
    treasury: Address,
    decimals: u32,
    name: Bytes,
    symbol: Bytes,
) {
    let topics = (Symbol::short("init"), underlying_asset, pool);
    e.events()
        .publish(topics, (treasury, decimals, name, symbol));
}
