use soroban_sdk::{symbol_short, Address, Env, String, Symbol};

pub(crate) fn initialized(
    e: &Env,
    underlying_asset: Address,
    pool: Address,
    decimals: u32,
    name: String,
    symbol: String,
) {
    let topics = (symbol_short!("init"), underlying_asset, pool);
    e.events().publish(topics, (decimals, name, symbol));
}

pub(crate) fn set_authorized(e: &Env, id: Address, authorize: bool) {
    let topics = (Symbol::new(e, "set_authorized"), id);
    e.events().publish(topics, authorize);
}

pub(crate) fn mint(e: &Env, admin: Address, to: Address, amount: i128) {
    let topics = (symbol_short!("mint"), admin, to);
    e.events().publish(topics, amount);
}

pub(crate) fn burn(e: &Env, from: Address, amount: i128) {
    let topics = (symbol_short!("burn"), from);
    e.events().publish(topics, amount);
}

pub(crate) fn clawback(e: &Env, from: Address, amount: i128) {
    let topics = (symbol_short!("clawback"), from);
    e.events().publish(topics, amount);
}
