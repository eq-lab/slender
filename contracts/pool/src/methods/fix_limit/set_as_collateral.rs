use pool_interface::types::error::Error;
use soroban_sdk::{assert_with_error, Address, Env};

use crate::methods::fix_limit::account_position::calc_account_data;
use crate::methods::utils::validation::require_good_position;
use crate::storage::read_reserve;
use crate::types::user_configurator::UserConfigurator;

pub fn set_as_collateral(
    env: &Env,
    who: &Address,
    asset: &Address,
    use_as_collateral: bool,
) -> Result<(), Error> {
    who.require_auth();

    let mut user_configurator = UserConfigurator::new(env, who, false);
    let user_config = user_configurator.user_config()?;
    let reserve_id = read_reserve(env, asset)?.get_id();

    assert_with_error!(
        env,
        !user_config.is_borrowing(env, reserve_id),
        Error::MustNotHaveDebt
    );

    if !use_as_collateral
        && user_config.is_borrowing_any()
        && user_config.is_using_as_collateral(env, reserve_id)
    {
        user_configurator.withdraw(reserve_id, asset, true)?;
        let user_config = user_configurator.user_config()?;
        let account_data = calc_account_data(env, who, None, None, None, None, user_config, false)?;

        require_good_position(env, &account_data);

        user_configurator.write();

        return Ok(());
    }

    user_configurator
        .deposit(reserve_id, asset, use_as_collateral)?
        .withdraw(reserve_id, asset, !use_as_collateral)?
        .write();

    Ok(())
}
