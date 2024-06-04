use pool_interface::types::{error::Error, permission::Permission};
use soroban_sdk::{Address, Env};

use crate::{read_permission_owners, write_permission_owners};

use super::utils::validation::require_permission;

pub fn revoke_permission(
    env: &Env,
    who: &Address,
    owner: &Address,
    permission: &Permission,
) -> Result<(), Error> {
    require_permission(env, who, &Permission::Permisssion)?;

    let mut permission_owners = read_permission_owners(env, permission);

    match permission_owners.binary_search(owner) {
        Ok(idx) => {
            permission_owners.remove(idx);
            write_permission_owners(env, &permission_owners, permission);
        }
        Err(_) => (),
    }

    Ok(())
}
