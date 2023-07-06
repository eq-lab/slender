use soroban_sdk::{Address, Bytes, Env, Symbol};

pub(crate) fn initialized(
    e: &Env,
    underlying_asset: Address,
    pool: Address,
    decimals: u32,
    name: Bytes,
    symbol: Bytes,
) {
    let topics = (Symbol::short("init"), underlying_asset, pool);
    e.events().publish(topics, (decimals, name, symbol));
}

pub(crate) fn set_authorized(e: &Env, id: Address, authorize: bool) {
    let topics = (Symbol::new(e, "set_authorized"), id);
    e.events().publish(topics, authorize);
}
