use pool_interface::types::{
    collateral_params_input::CollateralParamsInput, pool_config::PoolConfig,
};
use soroban_sdk::{symbol_short, Address, Env, Symbol};

pub(crate) fn initialized(e: &Env, admin: &Address, pool_config: &PoolConfig) {
    let topics = (
        Symbol::new(e, "initialize"),
        admin,
        pool_config.base_asset_address.clone(),
    );
    e.events().publish(
        topics,
        (
            pool_config.ir_alpha,
            pool_config.ir_initial_rate,
            pool_config.ir_max_rate,
            pool_config.ir_scaling_coeff,
            pool_config.base_asset_decimals,
            pool_config.initial_health,
            pool_config.grace_period,
            pool_config.timestamp_window,
            pool_config.flash_loan_fee,
            pool_config.user_assets_limit,
            pool_config.min_collat_amount,
            pool_config.min_debt_amount,
            pool_config.liquidation_protocol_fee,
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

pub(crate) fn reserve_status_changed(e: &Env, asset: &Address, activated: bool) {
    let topics = (asset.clone(),);
    e.events().publish(topics, activated);
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
    borrow: bool,
) {
    let topics = (Symbol::new(e, "flash_loan"), who, receiver, asset);
    e.events().publish(topics, (amount, premium, borrow));
}
