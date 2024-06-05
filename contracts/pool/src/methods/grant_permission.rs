use pool_interface::types::{error::Error, permission::Permission};
use soroban_sdk::{Address, Env};

use crate::{read_permission_owners, write_permission_owners};

use super::utils::validation::require_permission;

pub fn grant_permission(
    env: &Env,
    who: &Address,
    receiver: &Address,
    permission: &Permission,
) -> Result<(), Error> {
    require_permission(env, who, &Permission::Permission)?;

    let mut permission_owners = read_permission_owners(env, permission);

    match permission_owners.binary_search(receiver) {
        Ok(_) => (),
        Err(idx) => {
            permission_owners.insert(idx, receiver.clone());
            write_permission_owners(env, &permission_owners, permission);
        }
    }

    Ok(())
}
