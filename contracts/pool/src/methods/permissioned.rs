use pool_interface::types::{error::Error, permission::Permission};
use soroban_sdk::{Address, Env};

use crate::read_permission_owners;

pub fn permissioned(env: &Env, who: &Address, permission: &Permission) -> Result<bool, Error> {
    let permission_owners = read_permission_owners(env, permission);
    match permission_owners.binary_search(who) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
