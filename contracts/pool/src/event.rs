use pool_interface::types::collateral_params_input::CollateralParamsInput;
use pool_interface::types::ir_params::IRParams;
use soroban_sdk::{symbol_short, Address, Env, Symbol};

pub(crate) fn initialized(e: &Env, admin: &Address, treasury: &Address, params: &IRParams) {
    let topics = (Symbol::new(e, "initialize"), admin, treasury);
    e.events().publish(
        topics,
        (
            params.alpha,
            params.initial_rate,
            params.max_rate,
            params.scaling_coeff,
        ),
    );
}

pub(crate) fn reserve_used_as_collateral_enabled(e: &Env, who: &Address, asset: &Address) {
    let topics = (Symbol::new(e, "reserve_used_as_coll_enabled"), who.clone());
    e.events().publish(topics, asset.clone());
}

pub(crate) fn reserve_used_as_collateral_disabled(e: &Env, who: &Address, asset: &Address) {
    let topics = (Symbol::new(e, "reserve_used_as_coll_disabled"), who.clone());
    e.events().publish(topics, asset.clone());
}

pub(crate) fn deposit(e: &Env, who: &Address, asset: &Address, amount: i128) {
    let topics = (symbol_short!("deposit"), who.clone());
    e.events().publish(topics, (asset.clone(), amount));
}

pub(crate) fn withdraw(e: &Env, who: &Address, asset: &Address, to: &Address, amount: i128) {
    let topics = (symbol_short!("withdraw"), who.clone());
    e.events().publish(topics, (to, asset.clone(), amount));
}

pub(crate) fn borrow(e: &Env, who: &Address, asset: &Address, amount: i128) {
    let topics = (symbol_short!("borrow"), who.clone());
    e.events().publish(topics, (asset.clone(), amount));
}

pub(crate) fn repay(e: &Env, who: &Address, asset: &Address, amount: i128) {
    let topics = (symbol_short!("repay"), who.clone());
    e.events().publish(topics, (asset.clone(), amount));
}

pub(crate) fn collat_config_change(e: &Env, asset: &Address, params: &CollateralParamsInput) {
    let topics = (Symbol::new(e, "collat_config_change"), asset.clone());
    e.events().publish(
        topics,
        (
            params.liq_cap,
            params.pen_order,
            params.util_cap,
            params.discount,
        ),
    );
}

pub(crate) fn borrowing_enabled(e: &Env, asset: &Address) {
    let topics = (Symbol::new(e, "borrowing_enabled"), asset.clone());
    e.events().publish(topics, ());
}

pub(crate) fn borrowing_disabled(e: &Env, asset: &Address) {
    let topics = (Symbol::new(e, "borrowing_disabled"), asset.clone());
    e.events().publish(topics, ());
}

pub(crate) fn reserve_activated(e: &Env, asset: &Address) {
    let topics = (Symbol::new(e, "reserve_activated"), asset.clone());
    e.events().publish(topics, ());
}

pub(crate) fn reserve_deactivated(e: &Env, asset: &Address) {
    let topics = (Symbol::new(e, "reserve_deactivated"), asset.clone());
    e.events().publish(topics, ());
}

pub(crate) fn liquidation(e: &Env, who: &Address, covered_debt: i128, liquidated_collateral: i128) {
    let topics = (Symbol::new(e, "liquidation"), who.clone());
    e.events()
        .publish(topics, (covered_debt, liquidated_collateral));
}

pub(crate) fn flash_loan(
    e: &Env,
    who: &Address,
    receiver: &Address,
    asset: &Address,
    amount: i128,
    premium: i128,
) {
    let topics = (Symbol::new(e, "flash_loan"), who, receiver, asset);
    e.events().publish(topics, (amount, premium));
}
