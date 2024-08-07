use common::FixedI128;
use common::ONE_DAY;
use common::PERCENTAGE_FACTOR;
use pool_interface::types::collateral_params_input::CollateralParamsInput;
use pool_interface::types::error::Error;
use pool_interface::types::pause_info::PauseInfo;
use pool_interface::types::pool_config::PoolConfig;
use pool_interface::types::reserve_data::ReserveData;
use pool_interface::types::reserve_type::ReserveType;
use pool_interface::types::user_config::UserConfiguration;
use soroban_sdk::{assert_with_error, panic_with_error, Address, Env};

use crate::storage::{has_admin, read_admin};
use crate::types::account_data::AccountData;
use crate::{read_reserve, read_reserves};

pub fn require_admin_not_exist(env: &Env) {
    if has_admin(env) {
        panic_with_error!(env, Error::AlreadyInitialized);
    }
}

pub fn require_admin(env: &Env) -> Result<(), Error> {
    let admin: Address = read_admin(env)?;
    admin.require_auth();
    Ok(())
}

pub fn require_valid_collateral_params(env: &Env, params: &CollateralParamsInput) {
    require_lte_percentage_factor(env, params.discount);
    require_lte_percentage_factor(env, params.util_cap);
    assert_with_error!(env, params.liq_cap > 0, Error::BellowMinValue);
}

pub fn require_lte_percentage_factor(env: &Env, value: u32) {
    assert_with_error!(
        env,
        value <= PERCENTAGE_FACTOR,
        Error::MustBeLtePercentageFactor
    );
}

pub fn require_lt_percentage_factor(env: &Env, value: u32) {
    assert_with_error!(
        env,
        value < PERCENTAGE_FACTOR,
        Error::MustBeLtPercentageFactor
    );
}

pub fn require_gt_percentage_factor(env: &Env, value: u32) {
    assert_with_error!(
        env,
        value > PERCENTAGE_FACTOR,
        Error::MustBeGtPercentageFactor
    );
}

pub fn require_non_negative(env: &Env, value: i128) {
    assert_with_error!(env, value >= 0, Error::MustBeNonNegative);
}

pub fn require_positive_amount(env: &Env, amount: i128) {
    assert_with_error!(env, amount > 0, Error::InvalidAmount);
}

pub fn require_active_reserve(env: &Env, reserve: &ReserveData) {
    assert_with_error!(env, reserve.configuration.is_active, Error::NoActiveReserve);
}

pub fn require_borrowing_enabled(env: &Env, reserve: &ReserveData) {
    assert_with_error!(
        env,
        reserve.configuration.borrowing_enabled,
        Error::BorrowingDisabled
    );
}

pub fn require_liquidity_cap_not_exceeded(
    env: &Env,
    reserve: &ReserveData,
    debt_token_supply: i128,
    balance: i128,
    deposit_amount: i128,
) -> Result<(), Error> {
    let balance_after_deposit = FixedI128::from_inner(reserve.lender_ar)
        .mul_int(debt_token_supply)
        .ok_or(Error::MathOverflowError)?
        .checked_add(deposit_amount)
        .ok_or(Error::MathOverflowError)?
        .checked_add(balance)
        .ok_or(Error::MathOverflowError)?;

    assert_with_error!(
        env,
        balance_after_deposit <= reserve.configuration.liquidity_cap,
        Error::ExceededMaxValue
    );

    Ok(())
}

pub fn require_util_cap_not_exceeded(
    env: &Env,
    s_token_supply: i128,
    debt_token_supply: i128,
    util_cap: u32,
    amount: i128,
) -> Result<(), Error> {
    let debt_token_supply_after = debt_token_supply
        .checked_add(amount)
        .ok_or(Error::ValidateBorrowMathError)?;
    let utilization = FixedI128::from_rational(debt_token_supply_after, s_token_supply)
        .ok_or(Error::ValidateBorrowMathError)?;
    let util_cap = FixedI128::from_percentage(util_cap).ok_or(Error::ValidateBorrowMathError)?;

    assert_with_error!(env, utilization <= util_cap, Error::ExceededMaxValue);

    Ok(())
}

pub fn require_gte_initial_health(
    env: &Env,
    account_data: &AccountData,
    pool_config: &PoolConfig,
) -> Result<(), Error> {
    if account_data.npv == 0 && account_data.discounted_collateral == 0 {
        return Ok(());
    }

    assert_with_error!(
        env,
        account_data.npv >= 0 && account_data.discounted_collateral >= 0,
        Error::BellowMinValue
    );

    let npv_after_percent =
        FixedI128::from_rational(account_data.npv, account_data.discounted_collateral)
            .ok_or(Error::MathOverflowError)?;
    let initial_health_percent =
        FixedI128::from_percentage(pool_config.initial_health).ok_or(Error::MathOverflowError)?;

    assert_with_error!(
        env,
        npv_after_percent >= initial_health_percent,
        Error::BellowMinValue
    );

    Ok(())
}

pub fn require_not_in_collateral_asset(env: &Env, collat_balance: i128) {
    assert_with_error!(env, collat_balance == 0, Error::MustNotBeInCollateralAsset);
}

pub fn require_not_paused(env: &Env, pause_info: &PauseInfo) {
    assert_with_error!(env, !pause_info.paused, Error::Paused);
}

pub fn require_not_in_grace_period(env: &Env, pause_info: &PauseInfo) {
    let now = env.ledger().timestamp();
    assert_with_error!(
        env,
        now >= pause_info.grace_period_ends_at(),
        Error::GracePeriod
    );
}

pub fn require_debt(env: &Env, user_config: &UserConfiguration, reserve_id: u8) {
    assert_with_error!(
        env,
        user_config.is_borrowing(env, reserve_id),
        Error::DebtError
    );
}

pub fn require_zero_debt(env: &Env, user_config: &UserConfiguration, reserve_id: u8) {
    assert_with_error!(
        env,
        !user_config.is_borrowing(env, reserve_id),
        Error::DebtError
    );
}

pub fn require_fungible_reserve(env: &Env, reserve: &ReserveData) {
    assert_with_error!(
        env,
        matches!(reserve.reserve_type, ReserveType::Fungible(_, _)),
        Error::NotFungible
    );
}

pub fn require_unique_liquidation_order(
    env: &Env,
    asset: &Address,
    pen_order: u32,
) -> Result<(), Error> {
    for r_asset in read_reserves(env) {
        if r_asset.eq(asset) {
            continue;
        }

        let reserve = read_reserve(env, &r_asset)?;

        assert_with_error!(
            env,
            !reserve.configuration.pen_order.eq(&pen_order),
            Error::LiquidationOrderMustBeUnique
        );
    }

    Ok(())
}

pub fn require_not_exceed_assets_limit(env: &Env, assets_total: u32, assets_limit: u32) {
    assert_with_error!(env, assets_total <= assets_limit, Error::ExceededMaxValue);
}

pub fn require_min_position_amounts(
    env: &Env,
    account_data: &AccountData,
    pool_config: &PoolConfig,
) -> Result<(), Error> {
    if account_data.debt == 0 {
        return Ok(());
    }

    assert_with_error!(
        env,
        account_data.discounted_collateral >= pool_config.min_collat_amount,
        Error::BellowMinValue
    );
    assert_with_error!(
        env,
        account_data.debt >= pool_config.min_debt_amount,
        Error::BellowMinValue
    );

    Ok(())
}

pub fn require_valid_pool_config(env: &Env, config: &PoolConfig) {
    require_lte_percentage_factor(env, config.initial_health);
    require_lte_percentage_factor(env, config.flash_loan_fee);
    require_lte_percentage_factor(env, config.liquidation_protocol_fee);
    require_non_negative(env, config.min_collat_amount);
    require_non_negative(env, config.min_debt_amount);
    require_lte_percentage_factor(env, config.ir_initial_rate);
    require_gt_percentage_factor(env, config.ir_max_rate);
    require_lt_percentage_factor(env, config.ir_scaling_coeff);

    assert_with_error!(env, config.ir_scaling_coeff > 0, Error::MustBeNonNegative);
    assert_with_error!(
        env,
        config.ir_initial_rate <= config.ir_max_rate,
        Error::ExceededMaxValue
    );

    assert_with_error!(
        env,
        config.base_asset_decimals <= 38,
        Error::ExceededMaxValue
    );
    assert_with_error!(env, config.grace_period != 0, Error::BellowMinValue);
    assert_with_error!(env, config.grace_period <= ONE_DAY, Error::ExceededMaxValue);
    assert_with_error!(
        env,
        config.timestamp_window <= ONE_DAY,
        Error::ExceededMaxValue
    );
    assert_with_error!(env, config.user_assets_limit > 0, Error::BellowMinValue);
}
