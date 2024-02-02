use pool_interface::types::error::Error;
use soroban_sdk::{Address, Env};

use crate::event;
use crate::storage::{read_reserve, write_reserve};

use super::utils::validation::{require_admin, require_fungible_reserve};

pub fn enable_borrowing_on_reserve(env: &Env, asset: &Address, enabled: bool) -> Result<(), Error> {
    require_admin(env)?;

    let mut reserve = read_reserve(env, asset)?;

    if enabled {
        require_fungible_reserve(env, &reserve);
    }

    reserve.configuration.borrowing_enabled = enabled;
    write_reserve(env, asset, &reserve);

    if enabled {
        event::borrowing_enabled(env, asset);
    } else {
        event::borrowing_disabled(env, asset);
    }

    Ok(())
}
