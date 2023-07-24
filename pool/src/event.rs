use pool_interface::{CollateralParamsInput, IRParams};
use soroban_sdk::{Address, Env, Symbol};

pub(crate) fn initialized(e: &Env, admin: Address, treasury: Address, ir_params: IRParams) {
    let topics = (Symbol::new(e, "initialize"), admin, treasury);
    e.events().publish(topics, ir_params);
}

pub(crate) fn reserve_used_as_collateral_enabled(e: &Env, who: Address, asset: Address) {
    let topics = (Symbol::new(e, "reserve_used_as_coll_enabled"), who);
    e.events().publish(topics, asset);
}

pub(crate) fn reserve_used_as_collateral_disabled(e: &Env, who: Address, asset: Address) {
    let topics = (Symbol::new(e, "reserve_used_as_coll_disabled"), who);
    e.events().publish(topics, asset);
}

pub(crate) fn deposit(e: &Env, who: Address, asset: Address, amount: i128) {
    let topics = (Symbol::short("deposit"), who);
    e.events().publish(topics, (asset, amount));
}

pub(crate) fn withdraw(e: &Env, who: Address, asset: Address, to: Address, amount: i128) {
    let topics = (Symbol::short("withdraw"), who);
    e.events().publish(topics, (to, asset, amount));
}

pub(crate) fn borrow(e: &Env, who: Address, asset: Address, amount: i128) {
    let topics = (Symbol::short("borrow"), who);
    e.events().publish(topics, (asset, amount));
}

pub(crate) fn repay(e: &Env, who: Address, asset: Address, amount: i128) {
    let topics = (Symbol::short("repay"), who);
    e.events().publish(topics, (asset, amount));
}

pub(crate) fn collat_config_change(e: &Env, asset: Address, params: CollateralParamsInput) {
    let topics = (Symbol::new(e, "collat_config_change"), asset);
    e.events().publish(
        topics,
        (
            params.liq_bonus,
            params.liq_cap,
            params.util_cap,
            params.discount,
        ),
    );
}

pub(crate) fn borrowing_enabled(e: &Env, asset: Address) {
    let topics = (Symbol::new(e, "borrowing_enabled"), asset);
    e.events().publish(topics, ());
}

pub(crate) fn borrowing_disabled(e: &Env, asset: Address) {
    let topics = (Symbol::new(e, "borrowing_disabled"), asset);
    e.events().publish(topics, ());
}

pub(crate) fn liquidation(e: &Env, who: Address, covered_debt: i128, liquidated_collateral: i128) {
    let topics = (Symbol::new(e, "liquidation"), who);
    e.events()
        .publish(topics, (covered_debt, liquidated_collateral));
}
